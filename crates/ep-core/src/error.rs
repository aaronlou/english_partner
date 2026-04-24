use thiserror::Error;

#[derive(Error, Debug)]
pub enum EpError {
    #[error("audio error: {0}")]
    Audio(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("speech API error: {0}")]
    SpeechApi(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("invalid configuration: {0}")]
    Config(String),
}

pub type EpResult<T> = Result<T, EpError>;
