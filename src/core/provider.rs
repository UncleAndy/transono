#[async_trait]
pub trait Provider {

    type ClientEvent;

    type ServerEvent;

    async fn connect(&mut self) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn send(
        &mut self,
        event: Self::ClientEvent,
    ) -> Result<()>;

    async fn next_event(
        &mut self,
    ) -> Result<Self::ServerEvent>;
}
