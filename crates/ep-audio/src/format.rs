//! Audio format definitions and conversions.

/// Standard format used throughout the pipeline.
pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;
pub const BITS_PER_SAMPLE: u16 = 16;

/// A chunk of PCM audio data.
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<i16>,
    pub timestamp_ms: u64,
}

impl AudioChunk {
    pub fn duration_ms(&self) -> u64 {
        (self.samples.len() as u64 * 1000) / SAMPLE_RATE as u64
    }
}
