//! Microphone audio capture.

use crate::format::{AudioChunk, SAMPLE_RATE, CHANNELS};
use ep_core::EpResult;

/// Async audio capture stream.
pub struct AudioCapture {
    // TODO: cpal integration
}

impl AudioCapture {
    pub async fn start() -> EpResult<Self> {
        todo!("integrate cpal for microphone capture")
    }

    pub async fn next_chunk(&mut self) -> Option<AudioChunk> {
        todo!()
    }
}
