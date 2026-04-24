//! Soniox API client for Rust.
//!
//! Since Soniox does not provide an official Rust SDK, this crate implements
//! the REST and WebSocket APIs directly.
//!
//! APIs covered:
//! - TTS REST: POST /v1/tts/generate
//! - TTS WebSocket: wss://tts-rt.soniox.com/generate-websocket
//! - STT REST: POST /v1/transcriptions
//! - STT WebSocket: wss://stt-rt.soniox.com/transcribe-websocket

pub mod client;
pub mod tts;
pub mod stt;
pub mod types;

pub use client::*;
pub use tts::*;
pub use stt::*;
pub use types::*;
