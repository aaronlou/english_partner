//! Audio format definitions and conversions.
//!
//! `AudioChunk` and standard constants are now defined in `ep-core::speech`
//! so they can be shared across the entire workspace (audio, VAD, STT, etc.).

pub use ep_core::speech::{AudioChunk, BITS_PER_SAMPLE, CHANNELS, SAMPLE_RATE};
