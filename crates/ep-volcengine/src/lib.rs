//! Volcengine (ByteDance / 火山引擎) Realtime Speech API client.
//!
//! Provides a WebSocket client for the end-to-end real-time speech dialogue
//! API (`wss://openspeech.bytedance.com/api/v3/realtime/dialogue`).

pub mod binary;
pub mod client;
pub mod events;

pub use client::{ServerEvent, VolcengineClient};