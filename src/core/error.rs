use thiserror::Error;
use crate::audio::EncodedAudioFormat;

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {

    #[error(transparent)]
    Transport(#[from] TransportError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error("Audio processing error: {0}")]
    Processing(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("CPAL error: {0}")]
    Cpal(String),

    #[error(transparent)]
    Symphonia(#[from] symphonia::core::errors::Error),

    #[error(transparent)]
    Hound(#[from] hound::Error),

    #[error("Audio buffer full")]
    AudioBufferFull,

    #[error("Unsupported audio format")]
    UnsupportedAudioFormat(EncodedAudioFormat),

    #[error("Unsupported channel layout")]
    UnsupportedChannelLayout(),

    #[error(transparent)]
    Pipewire(#[from] pipewire::Error),

    #[error(transparent)]
    PodSerialize(#[from] cookie_factory::GenError),
}

impl From<&str> for CoreError {
    fn from(err: &str) -> Self {
        CoreError::Internal(err.to_string())
    }
}

impl From<String> for CoreError {
    fn from(err: String) -> Self {
        CoreError::Internal(err)
    }
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("connection closed")]
    ConnectionClosed,

    #[error(transparent)]
    WebSocket(
        #[from]
        tokio_tungstenite::tungstenite::Error,
    ),
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Protocol error: {0}")]
    Other(String),

    #[error(transparent)]
    Http(#[from] http::Error),

    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),

    #[error(transparent)]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Unexpected binary data")]
    UnexpectedBinaryData,
}
