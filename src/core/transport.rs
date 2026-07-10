use bytes::Bytes;
use async_trait::async_trait;

use crate::core::error::Result;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(
        &mut self,
        data: TransportData,
    ) -> Result<()>;

    async fn recv(
        &mut self,
    ) -> Result<TransportData>;

    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum TransportData {
    Text(Bytes),
    Binary(Bytes),
}


impl TransportData {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(data) => data.as_ref(),
            Self::Binary(data) => data.as_ref(),
        }
    }

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

pub fn serialize_bytes_as_str<S>(bytes: &Bytes, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let s = std::str::from_utf8(bytes.as_ref()).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(s)
}
