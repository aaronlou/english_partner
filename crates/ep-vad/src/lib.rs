//! Voice Activity Detection (VAD).
//!
//! Current implementation uses an energy-based detector with hysteresis
//! (de-bouncing). It is fast, dependency-free, and sufficient for MVP.
//!
//! Future evolution:
//! - Replace with `webrtc-vad` for better accuracy in noisy environments.
//! - Add neural VAD (e.g. Silero) for edge deployment.

/// VAD state machine output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    /// No speech detected.
    Silence,
    /// Speech just started (first frame above threshold after silence).
    SpeechStart,
    /// Speech is ongoing.
    Speech,
    /// Speech just ended (last frame below threshold after speech).
    SpeechEnd,
}

/// Energy-based voice activity detector with configurable hysteresis.
///
/// # Design
/// - Computes RMS energy per frame.
/// - Requires `speech_frames_needed` consecutive frames above threshold
///   to transition from `Silence` → `SpeechStart`.
/// - Requires `silence_frames_needed` consecutive frames below threshold
///   to transition from `Speech` → `SpeechEnd`.
///
/// This prevents spurious toggles caused by brief noise or breath sounds.
pub struct VadEngine {
    /// RMS energy threshold (for 16-bit PCM samples, range 0..=32767).
    threshold: i32,
    /// Consecutive speech frames required to trigger `SpeechStart`.
    speech_frames_needed: usize,
    /// Consecutive silence frames required to trigger `SpeechEnd`.
    silence_frames_needed: usize,

    // runtime counters
    speech_count: usize,
    silence_count: usize,
    prev_state: VadState,
}

impl VadEngine {
    /// Create a VAD engine with sensible defaults for 16 kHz, 30 ms frames.
    ///
    /// Default threshold = 800 (works well for typical headset/laptop mic).
    pub fn new() -> Self {
        Self {
            threshold: 800,
            speech_frames_needed: 3,
            silence_frames_needed: 10,
            speech_count: 0,
            silence_count: 0,
            prev_state: VadState::Silence,
        }
    }

    /// Set the energy threshold (higher = less sensitive).
    pub fn with_threshold(mut self, threshold: i32) -> Self {
        self.threshold = threshold.max(1);
        self
    }

    /// Set how many consecutive speech frames are needed to declare speech.
    pub fn with_speech_frames(mut self, n: usize) -> Self {
        self.speech_frames_needed = n.max(1);
        self
    }

    /// Set how many consecutive silence frames are needed to declare end.
    pub fn with_silence_frames(mut self, n: usize) -> Self {
        self.silence_frames_needed = n.max(1);
        self
    }

    /// Process a frame of mono 16-bit PCM samples.
    ///
    /// Returns the current VAD state. Pay attention to `SpeechStart` and
    /// `SpeechEnd` transitions — they signal the boundaries of an utterance.
    pub fn process(&mut self, samples: &[i16]) -> VadState {
        let energy = rms_energy(samples);
        let is_speech = energy >= self.threshold;

        let state = if is_speech {
            self.silence_count = 0;
            self.speech_count += 1;

            if self.speech_count >= self.speech_frames_needed {
                if self.prev_state == VadState::Silence || self.prev_state == VadState::SpeechEnd {
                    VadState::SpeechStart
                } else {
                    VadState::Speech
                }
            } else {
                // Still in hysteresis — report previous state
                self.prev_state
            }
        } else {
            self.speech_count = 0;
            self.silence_count += 1;

            if self.silence_count >= self.silence_frames_needed {
                if self.prev_state == VadState::Speech || self.prev_state == VadState::SpeechStart {
                    VadState::SpeechEnd
                } else {
                    VadState::Silence
                }
            } else {
                // Still in hysteresis
                self.prev_state
            }
        };

        self.prev_state = state;
        state
    }

    /// Reset internal counters (e.g. after a conversation turn ends).
    pub fn reset(&mut self) {
        self.speech_count = 0;
        self.silence_count = 0;
        self.prev_state = VadState::Silence;
    }
}

impl Default for VadEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute RMS energy scaled to i16 range.
fn rms_energy(samples: &[i16]) -> i32 {
    if samples.is_empty() {
        return 0;
    }
    let sum_sq: u64 = samples.iter().map(|s| (*s as i32).pow(2) as u64).sum();
    let mean_sq = sum_sq / samples.len() as u64;
    (mean_sq as f64).sqrt() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence_frame(len: usize) -> Vec<i16> {
        vec![0i16; len]
    }

    fn speech_frame(len: usize, amplitude: i16) -> Vec<i16> {
        // Sine wave at amplitude
        (0..len)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (len as f64 / 4.0);
                (angle.sin() * amplitude as f64) as i16
            })
            .collect()
    }

    #[test]
    fn test_silence_stays_silent() {
        let mut vad = VadEngine::new().with_threshold(100);
        let frame = silence_frame(480); // 30ms @ 16kHz
        for _ in 0..20 {
            assert_eq!(vad.process(&frame), VadState::Silence);
        }
    }

    #[test]
    fn test_speech_start_after_debounce() {
        let mut vad = VadEngine::new()
            .with_threshold(100)
            .with_speech_frames(3)
            .with_silence_frames(5);

        let silence = silence_frame(480);
        let speech = speech_frame(480, 2000);

        // Initial silence
        assert_eq!(vad.process(&silence), VadState::Silence);

        // 2 frames of speech — still hysteresis
        assert_eq!(vad.process(&speech), VadState::Silence);
        assert_eq!(vad.process(&speech), VadState::Silence);

        // 3rd frame — SpeechStart
        assert_eq!(vad.process(&speech), VadState::SpeechStart);

        // Continues
        assert_eq!(vad.process(&speech), VadState::Speech);
    }

    #[test]
    fn test_speech_end_after_debounce() {
        let mut vad = VadEngine::new()
            .with_threshold(100)
            .with_speech_frames(1)
            .with_silence_frames(3);

        let silence = silence_frame(480);
        let speech = speech_frame(480, 2000);

        assert_eq!(vad.process(&speech), VadState::SpeechStart);
        assert_eq!(vad.process(&speech), VadState::Speech);

        // 2 frames of silence — still hysteresis
        assert_eq!(vad.process(&silence), VadState::Speech);
        assert_eq!(vad.process(&silence), VadState::Speech);

        // 3rd frame — SpeechEnd
        assert_eq!(vad.process(&silence), VadState::SpeechEnd);

        // Then silence
        assert_eq!(vad.process(&silence), VadState::Silence);
    }

    #[test]
    fn test_reset() {
        let mut vad = VadEngine::new()
            .with_threshold(100)
            .with_speech_frames(1)
            .with_silence_frames(1);

        let speech = speech_frame(480, 2000);
        assert_eq!(vad.process(&speech), VadState::SpeechStart);

        vad.reset();
        assert_eq!(vad.prev_state, VadState::Silence);
    }
}
