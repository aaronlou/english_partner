//! Typed event definitions for the Volcengine Realtime Speech API.
//!
//! Client events (sent to server) and server events (received from server).

use serde::{Deserialize, Serialize};

// ── Event IDs ─────────────────────────────────────────────────────────────

/// Client event IDs.
pub mod client_event {
    pub const START_CONNECTION: u32 = 1;
    pub const FINISH_CONNECTION: u32 = 2;
    pub const START_SESSION: u32 = 100;
    pub const FINISH_SESSION: u32 = 102;
    pub const TASK_REQUEST: u32 = 200;
    pub const UPDATE_CONFIG: u32 = 201;
    pub const SAY_HELLO: u32 = 300;
    pub const END_ASR: u32 = 400;
    pub const CHAT_TTS_TEXT: u32 = 500;
    pub const CHAT_TEXT_QUERY: u32 = 501;
    pub const CHAT_RAG_TEXT: u32 = 502;
    pub const CONVERSATION_CREATE: u32 = 510;
    pub const CONVERSATION_UPDATE: u32 = 511;
    pub const CONVERSATION_RETRIEVE: u32 = 512;
    pub const CONVERSATION_TRUNCATE: u32 = 513;
    pub const CONVERSATION_DELETE: u32 = 514;
    pub const CLIENT_INTERRUPT: u32 = 515;
}

/// Server event IDs.
pub mod server_event {
    pub const CONNECTION_STARTED: u32 = 50;
    pub const CONNECTION_FAILED: u32 = 51;
    pub const CONNECTION_FINISHED: u32 = 52;
    pub const SESSION_STARTED: u32 = 150;
    pub const SESSION_FINISHED: u32 = 152;
    pub const SESSION_FAILED: u32 = 153;
    pub const USAGE_RESPONSE: u32 = 154;
    pub const CONFIG_UPDATED: u32 = 251;
    pub const TTS_SENTENCE_START: u32 = 350;
    pub const TTS_SENTENCE_END: u32 = 351;
    pub const TTS_RESPONSE: u32 = 352;
    pub const TTS_ENDED: u32 = 359;
    pub const ASR_INFO: u32 = 450;
    pub const ASR_RESPONSE: u32 = 451;
    pub const ASR_ENDED: u32 = 459;
    pub const CHAT_RESPONSE: u32 = 550;
    pub const CHAT_TEXT_QUERY_CONFIRMED: u32 = 553;
    pub const CHAT_ENDED: u32 = 559;
    pub const CONVERSATION_CREATED: u32 = 567;
    pub const CONVERSATION_UPDATED: u32 = 568;
    pub const CONVERSATION_RETRIEVED: u32 = 569;
    pub const CONVERSATION_TRUNCATED: u32 = 570;
    pub const CONVERSATION_DELETED: u32 = 571;
    pub const DIALOG_COMMON_ERROR: u32 = 599;
}

// ── Client Event Payloads ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Location {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub province: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub town: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            longitude: None, latitude: None, city: None,
            country: Some("中国".to_string()),
            province: None, district: None, town: None,
            country_code: Some("CN".to_string()),
            address: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsAudioConfig {
    pub channel: u32,
    pub format: String,
    pub sample_rate: u32,
}

impl Default for TtsAudioConfig {
    fn default() -> Self {
        Self {
            channel: 1,
            format: "pcm_s16le".to_string(),
            sample_rate: 24000,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TtsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    pub audio_config: TtsAudioConfig,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            speaker: Some("zh_female_vv_jupiter_bigtts".to_string()),
            audio_config: TtsAudioConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AsrAudioInfo {
    pub format: String,
    pub sample_rate: u32,
    pub channel: u32,
}

impl Default for AsrAudioInfo {
    fn default() -> Self {
        Self {
            format: "pcm".to_string(),
            sample_rate: 16000,
            channel: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DialogContextItem {
    pub role: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DialogExtra {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_audit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_volc_websearch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_mod: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DialogConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaking_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialog_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dialog_context: Option<Vec<DialogContextItem>>,
    pub extra: DialogExtra,
}

#[derive(Debug, Clone, Serialize)]
pub struct StartSessionPayload {
    pub tts: TtsConfig,
    pub asr: AsrPayload,
    pub dialog: DialogConfig,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AsrPayload {
    pub audio_info: AsrAudioInfo,
}

// ── Server Event Payloads ─────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SessionStartedPayload {
    pub dialog_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ErrorPayload {
    pub error: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AsrResponseResult {
    pub text: String,
    pub is_interim: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AsrResponsePayload {
    pub results: Vec<AsrResponseResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TtsSentenceStartPayload {
    #[serde(default)]
    pub tts_type: String,
    pub text: String,
    #[serde(default)]
    pub question_id: String,
    #[serde(default)]
    pub reply_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChatResponsePayload {
    pub content: String,
    #[serde(default)]
    pub question_id: String,
    #[serde(default)]
    pub reply_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UsagePayload {
    pub usage: UsageInfo,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UsageInfo {
    #[serde(default)]
    pub input_text_tokens: u64,
    #[serde(default)]
    pub input_audio_tokens: u64,
    #[serde(default)]
    pub output_text_tokens: u64,
    #[serde(default)]
    pub output_audio_tokens: u64,
}

/// Build a StartSession payload for English teaching.
pub fn english_teacher_config(scenario_name: &str, scenario_prompt: &str) -> StartSessionPayload {
    StartSessionPayload {
        tts: TtsConfig {
            speaker: Some("zh_female_vv_jupiter_bigtts".to_string()),
            audio_config: TtsAudioConfig {
                channel: 1,
                format: "pcm_s16le".to_string(),
                sample_rate: 24000,
            },
        },
        asr: AsrPayload::default(),
        dialog: DialogConfig {
            bot_name: Some(scenario_name.to_string()),
            system_role: Some(scenario_prompt.to_string()),
            speaking_style: Some("patient, encouraging, and warm. Speak slowly and clearly like a favorite teacher.".to_string()),
            dialog_id: None,
            location: Some(Location::default()),
            dialog_context: None,
            extra: DialogExtra {
                strict_audit: Some(false),
                audit_response: None,
                enable_volc_websearch: Some(false),
                input_mod: None,
                model: Some("1.2.1.1".to_string()),
            },
        },
    }
}