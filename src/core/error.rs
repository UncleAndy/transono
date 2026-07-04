use thiserror::Error;

pub type Result<T> =
std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {

    #[error("transport error")]
    Transport(#[from] TransportError),

    #[error("protocol error")]
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

#[derive(Debug, Error)]
pub enum ProtocolError {}
