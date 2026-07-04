#[async_trait]
pub trait Provider {

    async fn connect(&mut self) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn send(
        &mut self,
        command: ProviderCommand<'_>,
    ) -> Result<()>;

    async fn next_event(
        &mut self,
    ) -> Result<ProviderEvent>;
}
