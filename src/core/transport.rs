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

pub enum TransportData {
    Text(String),
    Binary(Vec<u8>),
}
