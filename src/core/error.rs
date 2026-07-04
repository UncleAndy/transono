use thiserror::Error;

pub type Result<T> =
std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {

    #[error(transparent)]
    Transport(#[from] TransportError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
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

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Protocol error: {0}")]
    Other(String),

    #[error(transparent)]
    Http(#[from] http::Error),

    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
