//! Real-time STT via Soniox WebSocket API.
//!
//! Protocol:
//! 1. Connect to wss://stt-rt.soniox.com/transcribe-websocket
//! 2. Send JSON config message
//! 3. Stream binary PCM s16le audio chunks
//! 4. Receive JSON responses with tokens (partial + final)
//! 5. Send empty binary frame to signal end of stream

use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::client::SonioxClient;

/// Events emitted by the STT stream.
#[derive(Debug, Clone)]
pub enum SttEvent {
    /// Live partial transcript (may change).
    Partial(String),
    /// Confirmed final transcript (will not change).
    Final(String),
    /// Endpoint detected — speaker likely finished talking.
    Endpoint,
    /// Stream finished and connection closed gracefully.
    Finished,
    /// Error occurred.
    Error(String),
}

#[derive(Debug, Clone, Serialize)]
struct SttConfig {
    api_key: String,
    model: String,
    audio_format: String,
    sample_rate: u32,
    num_channels: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_hints: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_endpoint_detection: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct Token {
    text: Option<String>,
    #[serde(default)]
    is_final: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SttResponse {
    #[serde(default)]
    tokens: Vec<Token>,
    #[serde(default)]
    endpoint: bool,
    #[serde(default)]
    finished: bool,
    #[serde(rename = "error_code", default, deserialize_with = "deserialize_string_or_int")]
    error_code: Option<String>,
    #[serde(rename = "error_message", default)]
    error_message: Option<String>,
}

fn deserialize_string_or_int<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        serde_json::Value::Null => Ok(None),
        _ => Err(D::Error::custom("expected string, number, or null")),
    }
}

/// A streaming STT session.
pub struct SttStream {
    audio_tx: mpsc::UnboundedSender<Vec<u8>>,
    _task: tokio::task::JoinHandle<()>,
}

impl SttStream {
    /// Create a new STT stream and connect to Soniox.
    ///
    /// Returns the stream handle and an event receiver.
    pub async fn new(client: &SonioxClient) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<SttEvent>)> {
        let url = &client.stt_ws_url;
        let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        let config = SttConfig {
            api_key: client.api_key.clone(),
            model: "stt-rt-v4".to_string(),
            audio_format: "pcm_s16le".to_string(),
            sample_rate: 16000,
            num_channels: 1,
            language_hints: Some(vec!["en".to_string()]),
            enable_endpoint_detection: Some(true),
        };
        write
            .send(Message::Text(serde_json::to_string(&config)?.into()))
            .await?;

        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<SttEvent>();

        let task = tokio::spawn(async move {
            let mut final_text = String::new();

            loop {
                tokio::select! {
                    Some(chunk) = audio_rx.recv() => {
                        let msg = if chunk.is_empty() {
                            Message::Binary(vec![].into())
                        } else {
                            Message::Binary(chunk.into())
                        };
                        if let Err(e) = write.send(msg).await {
                            let _ = event_tx.send(SttEvent::Error(e.to_string()));
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                tracing::debug!(raw = %text, "STT WebSocket message");
                                match serde_json::from_str::<SttResponse>(&text) {
                                    Ok(resp) => {
                                        if let Some(code) = resp.error_code {
                                            let _ = event_tx.send(SttEvent::Error(format!(
                                                "{}: {}",
                                                code,
                                                resp.error_message.unwrap_or_default()
                                            )));
                                            break;
                                        }

                                        let mut partial = String::new();
                                        for token in &resp.tokens {
                                            if let Some(text) = &token.text {
                                                if token.is_final {
                                                    final_text.push_str(text);
                                                } else {
                                                    partial.push_str(text);
                                                }
                                            }
                                        }

                                        if !partial.is_empty() {
                                            let _ = event_tx.send(SttEvent::Partial(format!(
                                                "{}{}",
                                                final_text, partial
                                            )));
                                        }

                                        if resp.endpoint {
                                            let _ = event_tx.send(SttEvent::Endpoint);
                                        }

                                        if resp.finished {
                                            let _ = event_tx.send(SttEvent::Final(final_text.clone()));
                                            let _ = event_tx.send(SttEvent::Finished);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(SttEvent::Error(format!(
                                            "JSON parse error: {}",
                                            e
                                        )));
                                    }
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                let _ = event_tx.send(SttEvent::Finished);
                                break;
                            }
                            Some(Err(e)) => {
                                let _ = event_tx.send(SttEvent::Error(e.to_string()));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        Ok((Self { audio_tx, _task: task }, event_rx))
    }

    /// Send a chunk of PCM s16le audio (little-endian 16-bit signed).
    pub fn send_audio(&self, chunk: Vec<u8>) {
        let _ = self.audio_tx.send(chunk);
    }

    /// Signal end of audio stream.
    pub fn finish(&self) {
        let _ = self.audio_tx.send(vec![]);
    }
}
