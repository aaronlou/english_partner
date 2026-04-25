//! Soniox API client for Rust.
//!
//! Since Soniox does not provide an official Rust SDK, this crate implements
//! the REST and WebSocket APIs directly.
//!
//! This crate implements the [`SpeechProvider`] trait from `ep-core`, making
//! Soniox swappable with any other speech backend (MiMo, Azure, local models).
//!
//! APIs covered:
//! - TTS REST: POST /v1/tts/generate
//! - TTS WebSocket: wss://tts-rt.soniox.com/generate-websocket *(planned)*
//! - STT REST: POST /v1/transcriptions
//! - STT WebSocket: wss://stt-rt.soniox.com/transcribe-websocket *(planned)*

pub mod client;
pub mod tts;
pub mod stt;
pub mod stt_stream;
pub mod types;

pub use client::*;
pub use types::*;
pub use stt_stream::*;
