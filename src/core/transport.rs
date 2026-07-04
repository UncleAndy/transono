use async_trait::async_trait;

use crate::core::error::Result;

#[async_trait]
pub trait Transport: Send + Sync {

    async fn connect(
        &mut self,
        url: &str,
    ) -> Result<()>;

    async fn send(
        &mut self,
        data: TransportData,
    ) -> Result<()>;

    async fn recv(
        &mut self,
    ) -> Result<TransportData>;

    async fn disconnect(
        &mut self,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum TransportData {
    Text(String),
    Binary(Vec<u8>),
}


impl TransportData {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Binary(data) => data.as_slice(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Text(text) => text.into_bytes(),
            Self::Binary(data) => data,
        }
    }
}

impl From<String> for TransportData {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for TransportData {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<Vec<u8>> for TransportData {
    fn from(value: Vec<u8>) -> Self {
        Self::Binary(value)
    }
}

impl AsRef<[u8]> for TransportData {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
