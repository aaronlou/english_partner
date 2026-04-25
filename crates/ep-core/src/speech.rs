//! Speech service abstractions (STT + TTS).
//!
//! This module defines the `SpeechProvider` trait — the central abstraction
//! that allows the application to swap between different speech backends
//! (Soniox, MiMo, Azure, local Whisper, etc.) without changing business logic.

use crate::EpResult;

// ---------------------------------------------------------------------------
// Format constants
// ---------------------------------------------------------------------------

/// Standard sample rate used throughout the pipeline (16 kHz).
pub const SAMPLE_RATE: u32 = 16000;

/// Mono audio.
pub const CHANNELS: u16 = 1;

/// 16-bit PCM.
pub const BITS_PER_SAMPLE: u16 = 16;

// ---------------------------------------------------------------------------
// Audio types
// ---------------------------------------------------------------------------

/// A chunk of raw PCM audio data (mono, 16-bit, 16 kHz).
///
/// Produced by the capture pipeline and consumed by VAD / STT.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    pub samples: Vec<i16>,
    pub timestamp_ms: u64,
}

impl AudioChunk {
    /// Duration of this chunk in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        (self.samples.len() as u64 * 1000) / SAMPLE_RATE as u64
    }

    /// Create an empty chunk at the given timestamp.
    pub fn empty(timestamp_ms: u64) -> Self {
        Self {
            samples: Vec::new(),
            timestamp_ms,
        }
    }
}

/// Complete audio payload for transmission or storage.
///
/// Wraps a WAV-encoded byte buffer with metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioData {
    pub wav_bytes: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl AudioData {
    pub fn new(wav_bytes: Vec<u8>, sample_rate: u32) -> Self {
        Self {
            wav_bytes,
            sample_rate,
            channels: CHANNELS,
            bits_per_sample: BITS_PER_SAMPLE,
        }
    }

    /// Duration in milliseconds (approximate, based on byte count).
    pub fn duration_ms(&self) -> u64 {
        let bytes_per_sample = (self.bits_per_sample / 8) as u64;
        let total_samples = self.wav_bytes.len() as u64 / bytes_per_sample;
        (total_samples * 1000) / self.sample_rate as u64
    }
}

// ---------------------------------------------------------------------------
// Transcript types
// ---------------------------------------------------------------------------

/// Result of a speech-to-text operation.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    pub text: String,
    /// `true` when the provider has determined the utterance is complete.
    pub is_final: bool,
    /// Confidence score in range [0.0, 1.0].
    pub confidence: f32,
}

impl Transcript {
    pub fn new(text: impl Into<String>, is_final: bool, confidence: f32) -> Self {
        Self {
            text: text.into(),
            is_final,
            confidence,
        }
    }
}

// ---------------------------------------------------------------------------
// TTS options
// ---------------------------------------------------------------------------

/// Configuration for text-to-speech synthesis.
#[derive(Debug, Clone, PartialEq)]
pub struct TtsOptions {
    pub voice: String,
    pub language: Option<String>,
    /// Playback speed multiplier (1.0 = normal).
    pub speed: f32,
    /// Optional model identifier (provider-specific).
    pub model: Option<String>,
}

impl TtsOptions {
    pub fn new(voice: impl Into<String>) -> Self {
        Self {
            voice: voice.into(),
            language: None,
            speed: 1.0,
            model: None,
        }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.clamp(0.5, 2.0);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl Default for TtsOptions {
    fn default() -> Self {
        Self::new("Adrian")
    }
}

// ---------------------------------------------------------------------------
// SpeechProvider trait
// ---------------------------------------------------------------------------

/// Unified interface for speech services.
///
/// Implementors:
/// - `ep_soniox::SonioxClient` — cloud STT/TTS via Soniox
/// - Future: `ep_mimo::MiMoSpeechClient`, `ep_azure::AzureSpeechClient`,
///   `ep_local::WhisperLocalClient`, etc.
///
/// The trait is `Send + Sync` so it can be shared across async tasks.
pub trait SpeechProvider: Send + Sync {
    /// Synthesize text into audio (WAV format).
    ///
    /// # Errors
    /// Returns `EpError::SpeechApi` on provider failures.
    fn synthesize(
        &self,
        text: &str,
        opts: TtsOptions,
    ) -> impl std::future::Future<Output = EpResult<AudioData>> + Send;

    /// Transcribe a complete audio clip to text.
    ///
    /// For real-time streaming, see the streaming extension planned in
    /// `ep-core/src/speech_stream.rs` (future work).
    ///
    /// # Errors
    /// Returns `EpError::SpeechApi` on provider failures.
    fn transcribe(
        &self,
        audio: AudioData,
    ) -> impl std::future::Future<Output = EpResult<Transcript>> + Send;
}
