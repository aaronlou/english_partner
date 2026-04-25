//! Audio pipeline: chunk buffering + VAD → speech utterances.
//!
//! Decouples raw microphone capture from downstream consumers (STT, etc.)
//! via an async channel boundary.

use ep_core::speech::AudioChunk;
use ep_vad::{VadEngine, VadState};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Synchronous state machine
// ---------------------------------------------------------------------------

/// Collects audio chunks and uses VAD to segment them into utterances.
///
/// # Usage
/// ```ignore
/// use ep_audio::pipeline::AudioPipeline;
/// use ep_vad::VadEngine;
///
/// let mut pipeline = AudioPipeline::new(VadEngine::new());
/// // for chunk in mic_chunks { ... }
/// ```
pub struct AudioPipeline {
    vad: VadEngine,
    /// Chunks collected during current speech segment.
    buffer: Vec<AudioChunk>,
    /// Whether we are currently inside a speech segment.
    in_speech: bool,
}

impl AudioPipeline {
    pub fn new(vad: VadEngine) -> Self {
        Self {
            vad,
            buffer: Vec::new(),
            in_speech: false,
        }
    }

    /// Process a single audio chunk.
    ///
    /// Returns `Some(utterance_chunks)` when VAD detects the end of speech.
    /// The returned vector contains all chunks from `SpeechStart` through
    /// `SpeechEnd`, inclusive.
    pub fn process(&mut self, chunk: AudioChunk) -> Option<Vec<AudioChunk>> {
        let state = self.vad.process(&chunk.samples);

        match state {
            VadState::SpeechStart => {
                self.buffer.clear();
                self.buffer.push(chunk);
                self.in_speech = true;
                None
            }
            VadState::Speech => {
                if self.in_speech {
                    self.buffer.push(chunk);
                }
                None
            }
            VadState::SpeechEnd => {
                if self.in_speech {
                    self.buffer.push(chunk);
                    self.in_speech = false;
                    let utterance = std::mem::take(&mut self.buffer);
                    Some(utterance)
                } else {
                    None
                }
            }
            VadState::Silence => {
                if !self.in_speech {
                    self.buffer.clear();
                }
                None
            }
        }
    }

    /// Force-flush the current buffer as an utterance.
    ///
    /// Useful when the user manually stops recording or a timeout fires.
    pub fn flush(&mut self) -> Option<Vec<AudioChunk>> {
        if self.buffer.is_empty() {
            return None;
        }
        self.in_speech = false;
        self.vad.reset();
        Some(std::mem::take(&mut self.buffer))
    }

    /// Reset the pipeline to its initial state.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.in_speech = false;
        self.vad.reset();
    }
}

// ---------------------------------------------------------------------------
// Async channel-based runner
// ---------------------------------------------------------------------------

/// Run the audio pipeline over async channels.
///
/// Reads `AudioChunk`s from `input`, applies VAD, and sends complete
/// utterances (as `Vec<AudioChunk>`) to `output`.
///
/// Returns when `input` is closed or `shutdown` receives a message.
pub async fn run_pipeline(
    vad: VadEngine,
    mut input: mpsc::Receiver<AudioChunk>,
    output: mpsc::Sender<Vec<AudioChunk>>,
    mut shutdown: mpsc::Receiver<()>,
) {
    let mut pipeline = AudioPipeline::new(vad);

    loop {
        tokio::select! {
            biased;

            _ = shutdown.recv() => {
                tracing::info!("audio_pipeline.shutdown");
                // Flush any remaining audio before exiting.
                if let Some(utterance) = pipeline.flush() {
                    let _ = output.send(utterance).await;
                }
                break;
            }

            maybe_chunk = input.recv() => {
                match maybe_chunk {
                    Some(chunk) => {
                        if let Some(utterance) = pipeline.process(chunk) {
                            tracing::debug!(
                                chunks = utterance.len(),
                                "audio_pipeline.utterance_detected"
                            );
                            if output.send(utterance).await.is_err() {
                                tracing::warn!("audio_pipeline.output_closed");
                                break;
                            }
                        }
                    }
                    None => {
                        tracing::info!("audio_pipeline.input_closed");
                        if let Some(utterance) = pipeline.flush() {
                            let _ = output.send(utterance).await;
                        }
                        break;
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use ep_core::speech::SAMPLE_RATE;

    fn chunk(samples: Vec<i16>, timestamp_ms: u64) -> AudioChunk {
        AudioChunk {
            samples,
            timestamp_ms,
        }
    }

    fn silence(samples: usize) -> Vec<i16> {
        vec![0i16; samples]
    }

    fn speech(samples: usize, amplitude: i16) -> Vec<i16> {
        (0..samples)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (samples as f64 / 4.0);
                (angle.sin() * amplitude as f64) as i16
            })
            .collect()
    }

    #[test]
    fn test_pipeline_silence_produces_nothing() {
        let mut pipeline = AudioPipeline::new(VadEngine::new().with_threshold(100));
        let frame = silence(480);

        for i in 0..20 {
            assert!(pipeline.process(chunk(frame.clone(), i * 30)).is_none());
        }
    }

    #[test]
    fn test_pipeline_detects_utterance() {
        let mut pipeline = AudioPipeline::new(
            VadEngine::new()
                .with_threshold(100)
                .with_speech_frames(1)
                .with_silence_frames(2),
        );

        let silence = silence(480);
        let speech = speech(480, 2000);

        // Silence
        assert!(pipeline.process(chunk(silence.clone(), 0)).is_none());

        // Speech start
        assert!(pipeline.process(chunk(speech.clone(), 30)).is_none());

        // More speech
        assert!(pipeline.process(chunk(speech.clone(), 60)).is_none());

        // Silence frame 1 — still in hysteresis
        assert!(pipeline.process(chunk(silence.clone(), 90)).is_none());

        // Silence frame 2 — SpeechEnd triggers utterance return
        let utterance = pipeline.process(chunk(silence.clone(), 120));
        assert!(utterance.is_some());
        let utterance = utterance.unwrap();
        assert_eq!(utterance.len(), 4); // speech start + 2 speech + speech end
    }

    #[test]
    fn test_pipeline_flush() {
        let mut pipeline = AudioPipeline::new(
            VadEngine::new()
                .with_threshold(100)
                .with_speech_frames(1)
                .with_silence_frames(10),
        );

        let speech = speech(480, 2000);
        pipeline.process(chunk(speech.clone(), 0));
        pipeline.process(chunk(speech.clone(), 30));

        // Not ended yet (silence_frames = 10)
        assert!(pipeline.process(chunk(speech.clone(), 60)).is_none());

        // Force flush
        let flushed = pipeline.flush();
        assert!(flushed.is_some());
        assert_eq!(flushed.unwrap().len(), 3);
    }
}
