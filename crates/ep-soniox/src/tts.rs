//! Text-to-Speech API implementations.

use crate::client::SonioxClient;
use crate::types::TtsRequest;

impl SonioxClient {
    /// Synchronous TTS via REST API.
    pub async fn tts_generate(&self, _req: TtsRequest) -> ep_core::EpResult<Vec<u8>> {
        // TODO: implement with reqwest once edition2024 compatibility is resolved
        todo!("REST TTS not yet implemented - waiting for reqwest compatibility")
    }

    // TODO: WebSocket streaming TTS
}
