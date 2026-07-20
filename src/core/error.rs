//! Error types shared across core, audio, and providers.

use thiserror::Error;
use crate::audio::EncodedAudioFormat;

/// Result alias used by core and line APIs.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Top-level library error.
///
/// Prefer mapping lower-level failures into these variants rather than
/// panicking on audio or session paths.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Transport-layer failure (disconnect, WebSocket I/O).
    #[error(transparent)]
    Transport(#[from] TransportError),

    /// Protocol encode/decode or HTTP construction failure.
    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    /// DSP or pipeline processing failure.
    #[error("Audio processing error: {0}")]
    Processing(String),

    /// Unexpected internal condition not covered by a more specific variant.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Underlying I/O failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// CPAL device or stream failure.
    #[error("CPAL error: {0}")]
    Cpal(String),

    /// Symphonia decode/demux failure.
    #[error(transparent)]
    Symphonia(#[from] symphonia::core::errors::Error),

    /// WAV (hound) read/write failure.
    #[error(transparent)]
    Hound(#[from] hound::Error),

    /// Ring or frame buffer rejected a write because it was full.
    #[error("Audio buffer full")]
    AudioBufferFull,

    /// Requested encoded audio format is not supported.
    #[error("Unsupported audio format")]
    UnsupportedAudioFormat(EncodedAudioFormat),

    /// Requested channel layout is not supported.
    #[error("Unsupported channel layout")]
    UnsupportedChannelLayout(),

    /// PipeWire client or graph operation failure.
    #[error(transparent)]
    Pipewire(#[from] pipewire::Error),

    /// SPA POD serialization failure.
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

/// Errors from bidirectional byte/text transports.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Peer closed the connection.
    #[error("connection closed")]
    ConnectionClosed,

    /// WebSocket protocol or I/O failure.
    #[error(transparent)]
    WebSocket(
        #[from]
        tokio_tungstenite::tungstenite::Error,
    ),
}

/// Protocol encode/decode or HTTP construction failures.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// Catch-all protocol failure with a human-readable message.
    #[error("Protocol error: {0}")]
    Other(String),

    /// HTTP request/response construction failure.
    #[error(transparent)]
    Http(#[from] http::Error),

    /// Invalid HTTP header value.
    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),

    /// Invalid HTTP header name.
    #[error(transparent)]
    InvalidHeaderName(#[from] http::header::InvalidHeaderName),

    /// JSON serialize/deserialize failure.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Received binary frames where text/JSON was expected.
    #[error("Unexpected binary data")]
    UnexpectedBinaryData,
}
