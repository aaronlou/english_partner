//! Text-to-Speech API implementations.

use crate::client::SonioxClient;
use crate::types::TtsRequest;
use ep_core::{AudioData, EpError, EpResult, SpeechProvider, Transcript, TtsOptions};

impl SonioxClient {
    /// Generate speech via Soniox REST TTS API.
    ///
    /// Returns raw audio bytes (WAV format).
    pub async fn tts_generate(&self, req: TtsRequest) -> EpResult<Vec<u8>> {
        let url = format!("{}/v1/tts/generate", self.base_url);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&req)
            .send()
            .await
            .map_err(|e| EpError::Network(format!("TTS request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(EpError::SpeechApi(format!(
                "Soniox TTS returned {status}: {body}"
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| EpError::Network(format!("TTS response read failed: {e}")))
    }
}

impl SpeechProvider for SonioxClient {
    async fn synthesize(&self, text: &str, opts: TtsOptions) -> EpResult<AudioData> {
        let req = TtsRequest {
            text: text.to_string(),
            model: opts.model.or_else(|| Some("tts-rt-v1-preview".to_string())),
            language: opts.language,
            voice: Some(opts.voice),
            audio_format: Some("wav".to_string()),
            sample_rate: Some(16000),
        };

        tracing::debug!(text = %text, "calling Soniox TTS");
        let wav_bytes = self.tts_generate(req).await?;
        tracing::debug!(bytes = wav_bytes.len(), "received TTS audio");

        Ok(AudioData::new(wav_bytes, 16000))
    }

    async fn transcribe(&self, audio: AudioData) -> EpResult<Transcript> {
        // Delegate to the dedicated STT helper defined in stt.rs.
        self.transcribe_rest(audio).await
    }
}
