//! Audio playback (TTS output).

use ep_core::EpResult;

pub struct AudioPlayback;

impl AudioPlayback {
    pub fn new() -> EpResult<Self> {
        Ok(Self)
    }

    pub async fn play(&mut self, _pcm_data: Vec<i16>) -> EpResult<()> {
        todo!("integrate rodio for audio playback")
    }
}
