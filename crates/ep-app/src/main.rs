//! English Partner — AI-powered English speaking practice via Volcengine Realtime API.
//!
//! Full-duplex voice conversation loop:
//!   Connect → StartSession with scenario → capture mic → send audio 20ms chunks
//!   → receive ASR/TTS events → play back AI speech → repeat

use std::io::{self, Write};
use std::time::Instant;

use ep_audio::stream::{AudioCapture, AudioPlayback};
use ep_volcengine::events;
use ep_volcengine::{ServerEvent, VolcengineClient};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

const PCM_CHUNK_BYTES: usize = 640; // 20ms @ 16kHz mono i16 LE

// ── Scenarios ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Scenario {
    name: &'static str,
    bot_name: &'static str,
    system_role: &'static str,
}

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "Restaurant Order",
        bot_name: "Waiter",
        system_role: "You are a friendly waiter at a Western restaurant. \
            The student is a Chinese primary school student practicing English. \
            Speak slowly and clearly. Use simple vocabulary (grade 3-6 level). \
            Gently correct grammar mistakes by repeating the correct form naturally. \
            Keep conversations engaging but under 5 turns. \
            If the student struggles, offer hints in simple English. \
            Always stay in character as a restaurant waiter.",
    },
    Scenario {
        name: "Self Introduction",
        bot_name: "Classmate",
        system_role: "You are a friendly new classmate meeting the student for the first time. \
            Help them practice introducing themselves in English. \
            Ask about their name, age, hobbies, and favorite subjects. \
            Use simple vocabulary. Praise their effort and gently correct mistakes. \
            Be warm and encouraging. If they get stuck, give simple hints.",
    },
    Scenario {
        name: "Shopping",
        bot_name: "Shop Assistant",
        system_role: "You are a helpful shop assistant at a toy store. \
            The student wants to buy a gift. Guide them through: \
            Greeting, asking about items, discussing prices, making a decision. \
            Use simple English and speak slowly. Be patient. Correct mistakes gently.",
    },
];

// ── PcmChunker ────────────────────────────────────────────────────────────

struct PcmChunker {
    buf: Vec<u8>,
}

impl PcmChunker {
    fn new() -> Self { Self { buf: Vec::new() } }

    fn push(&mut self, samples: &[i16]) -> Vec<Vec<u8>> {
        for &s in samples {
            self.buf.extend_from_slice(&s.to_le_bytes());
        }
        let mut out = Vec::new();
        while self.buf.len() >= PCM_CHUNK_BYTES {
            out.push(self.buf[..PCM_CHUNK_BYTES].to_vec());
            self.buf.drain(..PCM_CHUNK_BYTES);
        }
        out
    }
}

// ── Event processing state ────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum TurnState {
    Listening,
    WaitingForResponse,
    Playing,
}

// ── Main ──────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("info"))
        .init();

    println!();
    println!("╔══════════════════════════════════════════╗");
    println!("║   🤖 English Partner - AI 英语口语练习   ║");
    println!("║   Powered by Volcengine Realtime API    ║");
    println!("╚══════════════════════════════════════════╝");
    println!();

    // ── Credentials ──────────────────────────────────────────────────
    let app_id = std::env::var("VOLCANO_APPID")
        .map_err(|_| anyhow::anyhow!("VOLCANO_APPID env var not set"))?;
    let access_key = std::env::var("VOLCANO_ACCESS_TOKEN")
        .map_err(|_| anyhow::anyhow!("VOLCANO_ACCESS_TOKEN env var not set"))?;
    let app_key = std::env::var("VOLCANO_APP_KEY")
        .unwrap_or_else(|_| "PlgvMymc7f3tQnJ6".to_string());

    // ── Connect ──────────────────────────────────────────────────────
    print!("🔗 Connecting... ");
    io::stdout().flush()?;

    let client = VolcengineClient::connect(&app_id, &access_key, &app_key).await
        .map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;
    println!("connected!");

    let mut events = client.subscribe();

    // Wait for ConnectionStarted
    match events.recv().await {
        Ok(ServerEvent::ConnectionStarted) => info!("ConnectionStarted"),
        Ok(ServerEvent::Error { code, message }) => {
            anyhow::bail!("Connection error [{}]: {}", code, message);
        }
        other => anyhow::bail!("Unexpected: {:?}", other),
    }

    // ── Scenario ─────────────────────────────────────────────────────
    let scenario = choose_scenario();
    let session_id = format!("s{}", timestamp_ms());
    let config = make_config(&scenario);

    println!("\n🎭 {} | Session: {}", scenario.name, &session_id[..8]);
    print!("▶️  Starting session... ");
    io::stdout().flush()?;

    client.start_session(&session_id, &config)?;

    match events.recv().await {
        Ok(ServerEvent::SessionStarted { dialog_id }) => {
            info!(%dialog_id, "Session started");
            println!("ready!");
        }
        Ok(ServerEvent::Error { code, message }) => {
            anyhow::bail!("Session error [{}]: {}", code, message);
        }
        other => anyhow::bail!("Unexpected: {:?}", other),
    }

    // ── Audio I/O ────────────────────────────────────────────────────
    let mut mic = AudioCapture::new()
        .map_err(|e| anyhow::anyhow!("Mic error: {}", e))?;
    let playback = AudioPlayback::new()?;

    let mut chunker = PcmChunker::new();
    let mut state = TurnState::Listening;
    let mut pending_chunks: Vec<Vec<u8>> = Vec::new();
    let mut last_send = Instant::now();
    let send_interval = std::time::Duration::from_millis(20);
    let mut running = true;
    let mut last_heard_text = String::new();

    println!();
    println!("🎙️  Start speaking! (Ctrl+C to quit)");
println!("{}", "─".repeat(50));

    // ── Main loop ────────────────────────────────────────────────────
    while running {
        tokio::select! {
            // Server events
            result = events.recv() => {
                match result {
                    Ok(event) => {
                        match &event {
                            ServerEvent::AsrResponse { text, is_interim } => {
                                if *text != last_heard_text {
                                    let clear = " ".repeat(
                                        last_heard_text.len().saturating_sub(text.len())
                                    );
                                    print!("\r  👤 Hearing: {}...{}  ", text, clear);
                                    io::stdout().flush()?;
                                    last_heard_text = text.clone();
                                }
                                if !is_interim {
                                    println!();
                                    println!("  📝 You said: {}", text);
                                    last_heard_text.clear();
                                    state = TurnState::WaitingForResponse;
                                }
                            }
                            ServerEvent::AsrEnded => {
                                state = TurnState::WaitingForResponse;
                                println!("\n  ⏳ Thinking...");
                            }
                            ServerEvent::TtsSentenceStart { text, tts_type, .. } => {
                                if tts_type == "default" {
                                    println!("\n🤖 AI: {}\n", text);
                                }
                                state = TurnState::Playing;
                            }
                            ServerEvent::TtsAudio(audio) => {
                                let samples: Vec<i16> = audio
                                    .chunks_exact(2)
                                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                                    .collect();
                                playback.play_pcm(&samples, 24000);
                            }
                            ServerEvent::TtsEnded => {
                                state = TurnState::Listening;
println!("{}", "─".repeat(50));
                                println!("\n🎙️  Your turn! Speak now...");
                            }
                            ServerEvent::ChatResponse { content, .. } => {
                                if state == TurnState::WaitingForResponse {
                                    println!("🤖 AI: {}", content);
                                }
                            }
                            ServerEvent::Usage { input_text, input_audio, output_text, output_audio } => {
                                info!(input_text, input_audio, output_text, output_audio, "Tokens");
                            }
                            ServerEvent::Error { code, message } => {
                                error!(code, message, "Server error");
                                eprintln!("\n❌ Error [{}]: {}", code, message);
                            }
                            ServerEvent::SessionFinished => {
                                info!("Session finished by server");
                                running = false;
                            }
                            ServerEvent::ConnectionFinished => {
                                info!("Connection closed");
                                running = false;
                            }
                            _ => {
                                info!(?event, "Event");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        running = false;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Event channel lagged by {} messages", n);
                    }
                }
            }

            // Mic audio
            chunk = mic.recv() => {
                match chunk {
                    Some(audio) => {
                        let chunks = chunker.push(&audio.samples);
                        pending_chunks.extend(chunks);

                        let now = Instant::now();
                        if !pending_chunks.is_empty() && (now - last_send >= send_interval) {
                            for pcm in pending_chunks.drain(..) {
                                if state == TurnState::Listening {
                                    client.send_audio(&session_id, &pcm);
                                }
                            }
                            last_send = now;
                        }
                    }
                    None => {
                        warn!("Mic capture ended");
                        running = false;
                    }
                }
            }

            // Ctrl+C handler — poll every 500ms
            _ = tokio::signal::ctrl_c() => {
                println!("\n\n👋 Goodbye!");
                running = false;
            }
        }
    }

    // ── Cleanup ──────────────────────────────────────────────────────
    client.finish_session(&session_id);
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    client.finish_connection();
    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    println!("👋 Keep practicing! 🎓");
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn choose_scenario() -> Scenario {
    println!("📚 Scenarios:");
    for (i, s) in SCENARIOS.iter().enumerate() {
        println!("  {}. {}", i + 1, s.name);
    }
    let mut input = String::new();
    print!("\nChoose (1-{}): ", SCENARIOS.len());
    io::stdout().flush().ok();
    io::stdin().read_line(&mut input).ok();

    let idx: usize = input.trim().parse().unwrap_or(1);
    let idx = idx.saturating_sub(1).min(SCENARIOS.len() - 1);
    SCENARIOS[idx].clone()
}

fn make_config(scenario: &Scenario) -> events::StartSessionPayload {
    events::StartSessionPayload {
        tts: events::TtsConfig {
            speaker: Some("zh_female_vv_jupiter_bigtts".to_string()),
            audio_config: events::TtsAudioConfig {
                channel: 1,
                format: "pcm_s16le".to_string(),
                sample_rate: 24000,
            },
        },
        asr: events::AsrPayload::default(),
        dialog: events::DialogConfig {
            bot_name: Some(scenario.bot_name.to_string()),
            system_role: Some(scenario.system_role.to_string()),
            speaking_style: Some(
                "Patient, encouraging, and warm. Speak slowly and clearly with a natural \
                American English accent. You are a favorite teacher who makes learning fun. \
                Use simple vocabulary suitable for elementary school students.".to_string()
            ),
            dialog_id: None,
            location: Some(events::Location::default()),
            dialog_context: None,
            extra: events::DialogExtra {
                strict_audit: Some(false),
                audit_response: None,
                enable_volc_websearch: Some(false),
                input_mod: None,
                model: Some("1.2.1.1".to_string()),
            },
        },
    }
}

fn timestamp_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}