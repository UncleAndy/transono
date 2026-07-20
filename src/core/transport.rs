//! Bidirectional byte/text transport abstraction.
//!
//! High-level session logic depends on [`Transport`], not on WebSocket or
//! HTTP specifics. Concrete carriers (e.g. [`super::websocket::WebSocketTransport`])
//! implement this trait.

use bytes::Bytes;
use async_trait::async_trait;

use crate::core::error::Result;

/// Bidirectional duplex that sends and receives [`TransportData`].
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send one payload to the peer.
    ///
    /// # Errors
    ///
    /// Returns transport errors if the connection is closed or I/O fails.
    async fn send(
        &mut self,
        data: TransportData,
    ) -> Result<()>;

    /// Receive the next payload from the peer.
    ///
    /// # Errors
    ///
    /// Returns transport errors on disconnect or I/O failure.
    async fn recv(
        &mut self,
    ) -> Result<TransportData>;

    /// Close the transport cleanly.
    ///
    /// # Errors
    ///
    /// May return transport errors if teardown fails. Implementations may
    /// also treat close as best-effort and return `Ok(())` after attempting
    /// shutdown.
    async fn close(&mut self) -> Result<()>;
}

/// Unit of data exchanged over a [`Transport`].
#[derive(Debug, Clone)]
pub enum TransportData {
    /// UTF-8 text frame (e.g. JSON control messages).
    Text(Bytes),
    /// Opaque binary frame (e.g. encoded audio).
    Binary(Bytes),
}

impl TransportData {
    /// Borrow the underlying byte slice (text or binary).
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(data) => data.as_ref(),
            Self::Binary(data) => data.as_ref(),
        }
    }

    /// Consume this payload and return the owned [`Bytes`].
    pub fn into_bytes(self) -> Bytes {
        match self {
            Self::Text(data) => data,
            Self::Binary(data) => data,
        }
    }
}

impl From<String> for TransportData {
    fn from(value: String) -> Self {
        Self::Text(Bytes::from(value))
    }
}

impl From<&str> for TransportData {
    fn from(value: &str) -> Self {
        Self::Text(Bytes::copy_from_slice(value.as_bytes()))
    }
}

impl From<Vec<u8>> for TransportData {
    fn from(value: Vec<u8>) -> Self {
        Self::Binary(Bytes::from(value))
    }
}

impl From<Bytes> for TransportData {
    fn from(value: Bytes) -> Self {
        Self::Binary(value)
    }
}

impl AsRef<[u8]> for TransportData {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

/// Serde helper: serialize [`Bytes`] as a UTF-8 string.
///
/// # Errors
///
/// Fails the serializer if the bytes are not valid UTF-8.
pub fn serialize_bytes_as_str<S>(bytes: &Bytes, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = std::str::from_utf8(bytes.as_ref()).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(s)
}
