//! Speech-to-Text API implementations.

use crate::client::SonioxClient;
use ep_core::{AudioData, EpError, EpResult, Transcript};

impl SonioxClient {
    /// Transcribe audio via Soniox REST async API.
    ///
    /// Returns the full transcript once processing is complete.
    pub async fn transcribe_rest(&self, audio: AudioData) -> EpResult<Transcript> {
        let url = format!("{}/v1/transcriptions", self.base_url);

        let file_part = reqwest::multipart::Part::bytes(audio.wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| EpError::Config(format!("invalid mime type: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", "stt-async-v4")
            .text("audio_format", "wav");

        tracing::debug!("calling Soniox STT async");
        let resp = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| EpError::Network(format!("STT request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(EpError::SpeechApi(format!(
                "Soniox STT returned {status}: {body}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| EpError::Network(format!("STT JSON parse failed: {e}")))?;

        let text = json
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        tracing::debug!(text = %text, "received STT transcript");

        // For the async REST API, the result is always final.
        Ok(Transcript::new(text, true, 1.0))
    }
}
