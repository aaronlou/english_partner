//! Voice Activity Detection.
//!
//! Detects when the user starts and stops speaking.
//! Critical for turn-taking in conversational AI.

pub struct VadEngine;

impl VadEngine {
    pub fn new() -> Self {
        Self
    }

    /// Process audio frames and detect speech state transitions.
    pub fn process(&mut self, _samples: &[i16]) -> VadState {
        todo!("implement VAD logic")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    Silence,
    SpeechStart,
    Speech,
    SpeechEnd,
}
