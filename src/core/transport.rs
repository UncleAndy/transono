#[async_trait]
pub trait Transport {

    async fn connect(
        &mut self,
        endpoint: &str,
    ) -> Result<()>;

    async fn send(
        &mut self,
        text: String,
    ) -> Result<()>;

    async fn recv(
        &mut self,
    ) -> Result<String>;

    async fn disconnect(
        &mut self,
    ) -> Result<()>;
}
