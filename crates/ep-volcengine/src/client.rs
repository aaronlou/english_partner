//! Volcengine Realtime Speech API WebSocket client.
//!
//! Manages the full lifecycle: connection → session → audio streaming → teardown.
//!
//! Architecture:
//!   - `VolcengineClient` holds the connection and exposes a sender + receiver
//!   - `send_audio()` etc. can be called from ANY task concurrently via the sender
//!   - Server events are received through a `tokio::sync::broadcast` channel
//!   - Multiple receivers can be created via `subscribe()`
//!
//! Audio format (input):  PCM 16kHz mono 16-bit LE, 20ms frames (640 bytes each)
//! Audio format (output): PCM 24kHz mono 16-bit LE (configured in StartSession)

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{debug, error, info, trace, warn};

use crate::binary;
use crate::events::StartSessionPayload;

// ── Public Types ──────────────────────────────────────────────────────────

/// Events received from the server, parsed and typed.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    ConnectionStarted,
    ConnectionFinished,
    SessionStarted { dialog_id: String },
    SessionFinished,
    AsrResponse { text: String, is_interim: bool },
    AsrEnded,
    TtsSentenceStart { tts_type: String, text: String, question_id: String, reply_id: String },
    TtsAudio(Vec<u8>),
    TtsEnded,
    ChatResponse { content: String, question_id: String, reply_id: String },
    ChatEnded,
    Usage { input_text: u64, input_audio: u64, output_text: u64, output_audio: u64 },
    Error { code: u32, message: String },
    Unknown { event_id: u32 },
}

// ── Client ────────────────────────────────────────────────────────────────

/// The Volcengine realtime client.
///
/// After creation, use `subscribe()` to get a receiver for server events.
/// Use `send_*` methods to send commands to the server.
pub struct VolcengineClient {
    cmd_tx: mpsc::UnboundedSender<Vec<u8>>,
    event_tx: broadcast::Sender<ServerEvent>,
    connected: Arc<AtomicBool>,
    _task: tokio::task::JoinHandle<()>,
}

impl VolcengineClient {
    /// Connect to the Volcengine Realtime API.
    ///
    /// Sends StartConnection automatically. ConnectionStarted arrives
    /// on the event channel.
    pub async fn connect(app_id: &str, access_key: &str, app_key: &str) -> anyhow::Result<Self> {
        let url = "wss://openspeech.bytedance.com/api/v3/realtime/dialogue";
        let connect_id = uuid_v4();
        let request = http::Request::builder()
            .uri(url)
            .header("Host", "openspeech.bytedance.com")
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .header("Sec-WebSocket-Version", "13")
            .header("X-Api-App-Id", app_id)
            .header("X-Api-Access-Key", access_key)
            .header("X-Api-Resource-Id", "volc.speech.dialog")
            .header("X-Api-App-Key", app_key)
            .header("X-Api-Connect-Id", &connect_id)
            .body(())?;

        info!(connect_id = %connect_id, "Connecting to Volcengine Realtime API...");
        let (ws_stream, response) = tokio_tungstenite::connect_async(request).await?;
        info!(status = ?response.status(), "WebSocket handshake complete");

        if let Some(logid) = response.headers().get("X-Tt-Logid") {
            info!(logid = ?logid, "Server log ID");
        }

        let (mut write, mut read) = ws_stream.split();
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (event_tx, _) = broadcast::channel::<ServerEvent>(256);
        let event_tx_writer = event_tx.clone();
        let connected = Arc::new(AtomicBool::new(true));
        let connected_clone = connected.clone();

        // Send StartConnection
        let start_conn = binary::encode_connect_event(1, "{}");
        write.send(Message::Binary(start_conn.into())).await?;

        // Background task: writer + reader
        let task = tokio::spawn(async move {
            let writer = {
                let mut write = write;
                tokio::spawn(async move {
                    while let Some(data) = cmd_rx.recv().await {
                        if let Err(e) = write.send(Message::Binary(data.into())).await {
                            error!("WS write error: {}", e);
                            break;
                        }
                    }
                    let _ = write.close().await;
                })
            };

            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        let evt = match binary::decode_frame(&data) {
                            Ok(frame) => convert_frame(frame),
                            Err(e) => {
                                warn!("Frame decode error ({} bytes): {}", data.len(), e);
                                // Dump raw hex for debugging
                                let hex: String = data.iter()
                                    .take(64)
                                    .map(|b| format!("{:02X}", b))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                warn!("Raw frame hex (first 64B): {}", hex);
                                if let Ok(text) = std::str::from_utf8(&data) {
                                    warn!("Raw frame as text: {}", &text[..text.len().min(200)]);
                                }
                                continue;
                            }
                        };
                        match &evt {
                            ServerEvent::AsrResponse { .. } => {
                                trace!(?evt, "Server event");
                            }
                            _ => {
                                debug!(?evt, "Server event");
                            }
                        }
                        let _ = event_tx_writer.send(evt);
                    }
                    Ok(Message::Close(frame)) => {
                        info!(?frame, "WebSocket closed by server");
                        connected_clone.store(false, Ordering::SeqCst);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        connected_clone.store(false, Ordering::SeqCst);
                        break;
                    }
                    _ => {}
                }
            }
            writer.abort();
        });

        Ok(Self {
            cmd_tx,
            event_tx,
            connected,
            _task: task,
        })
    }

    /// Get a receiver for server events. Multiple subscribers are supported.
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.event_tx.subscribe()
    }

    /// Start a new session with the given configuration.
    pub fn start_session(&self, session_id: &str, config: &StartSessionPayload) -> anyhow::Result<()> {
        let json = serde_json::to_string(config)?;
        let frame = binary::encode_session_event(100, session_id, &json);
        self.send_raw(frame);
        Ok(())
    }

    /// Send a text query (ChatTextQuery event 501).
    pub fn send_text_query(&self, session_id: &str, text: &str) {
        let json = serde_json::json!({"content": text}).to_string();
        let frame = binary::encode_session_event(501, session_id, &json);
        self.send_raw(frame);
    }

    /// Send PCM audio data (TaskRequest event 200).
    /// `pcm` should be raw 16-bit LE, 16kHz, mono samples.
    pub fn send_audio(&self, session_id: &str, pcm: &[u8]) {
        let frame = binary::encode_audio_event(session_id, pcm);
        self.send_raw(frame);
    }

    /// Send FinishSession (event 102) to end the current session.
    pub fn finish_session(&self, session_id: &str) {
        let frame = binary::encode_session_event(102, session_id, "{}");
        self.send_raw(frame);
    }

    /// Send FinishConnection (event 2) to close the WebSocket.
    pub fn finish_connection(&self) {
        let frame = binary::encode_connect_event(2, "{}");
        self.send_raw(frame);
    }

    fn send_raw(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(data);
    }

    /// Check if the WebSocket is still connected.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}

impl Drop for VolcengineClient {
    fn drop(&mut self) {
        self.connected.store(false, Ordering::SeqCst);
    }
}

// ── Frame Conversion ──────────────────────────────────────────────────────

fn convert_frame(frame: binary::ServerFrame) -> ServerEvent {
    use binary::ServerFrame;

    match frame {
        ServerFrame::Json { event_id, payload, .. } => match event_id {
            50 => ServerEvent::ConnectionStarted,
            52 => ServerEvent::ConnectionFinished,
            150 => {
                let dialog_id = payload["dialog_id"].as_str().unwrap_or("").to_string();
                ServerEvent::SessionStarted { dialog_id }
            }
            152 => ServerEvent::SessionFinished,
            350 => ServerEvent::TtsSentenceStart {
                tts_type: payload["tts_type"].as_str().unwrap_or("").to_string(),
                text: payload["text"].as_str().unwrap_or("").to_string(),
                question_id: payload["question_id"].as_str().unwrap_or("").to_string(),
                reply_id: payload["reply_id"].as_str().unwrap_or("").to_string(),
            },
            359 => ServerEvent::TtsEnded,
            451 => {
                let results = payload["results"].as_array();
                let text = results
                    .and_then(|r| r.first())
                    .and_then(|r| r["text"].as_str())
                    .unwrap_or("")
                    .to_string();
                let is_interim = results
                    .and_then(|r| r.first())
                    .and_then(|r| r["is_interim"].as_bool())
                    .unwrap_or(false);
                ServerEvent::AsrResponse { text, is_interim }
            }
            459 => ServerEvent::AsrEnded,
            550 => ServerEvent::ChatResponse {
                content: payload["content"].as_str().unwrap_or("").to_string(),
                question_id: payload["question_id"].as_str().unwrap_or("").to_string(),
                reply_id: payload["reply_id"].as_str().unwrap_or("").to_string(),
            },
            559 => ServerEvent::ChatEnded,
            154 => {
                let u = &payload["usage"];
                ServerEvent::Usage {
                    input_text: u["input_text_tokens"].as_u64().unwrap_or(0),
                    input_audio: u["input_audio_tokens"].as_u64().unwrap_or(0),
                    output_text: u["output_text_tokens"].as_u64().unwrap_or(0),
                    output_audio: u["output_audio_tokens"].as_u64().unwrap_or(0),
                }
            }
            153 | 599 => ServerEvent::Error {
                code: payload["status_code"].as_u64().unwrap_or(0) as u32,
                message: payload["error"]
                    .as_str()
                    .or(payload["message"].as_str())
                    .unwrap_or("unknown")
                    .to_string(),
            },
            other => ServerEvent::Unknown { event_id: other },
        },
        ServerFrame::Audio { data, .. } => ServerEvent::TtsAudio(data),
        ServerFrame::Error { code, message } => ServerEvent::Error { code, message },
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:016x}", ts)
}