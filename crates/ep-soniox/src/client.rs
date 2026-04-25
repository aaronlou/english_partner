//! HTTP client configuration for Soniox API.

use reqwest::Client;

const DEFAULT_BASE_URL: &str = "https://api.soniox.com";
const DEFAULT_TTS_WS_URL: &str = "wss://tts-rt.soniox.com/generate-websocket";
const DEFAULT_STT_WS_URL: &str = "wss://stt-rt.soniox.com/transcribe-websocket";

pub struct SonioxClient {
    pub api_key: String,
    pub http: Client,
    pub base_url: String,
    pub tts_ws_url: String,
    pub stt_ws_url: String,
}

impl SonioxClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            http: Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            tts_ws_url: DEFAULT_TTS_WS_URL.to_string(),
            stt_ws_url: DEFAULT_STT_WS_URL.to_string(),
        }
    }

    pub fn with_region(mut self, region: &str) -> Self {
        match region {
            "eu" => {
                self.base_url = "https://api.eu.soniox.com".to_string();
                self.tts_ws_url = "wss://tts-rt.eu.soniox.com/generate-websocket".to_string();
                self.stt_ws_url = "wss://stt-rt.eu.soniox.com/transcribe-websocket".to_string();
            }
            "jp" => {
                self.base_url = "https://api.jp.soniox.com".to_string();
                self.tts_ws_url = "wss://tts-rt.jp.soniox.com/generate-websocket".to_string();
                self.stt_ws_url = "wss://stt-rt.jp.soniox.com/transcribe-websocket".to_string();
            }
            _ => {} // default US
        }
        self
    }

    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }
}
