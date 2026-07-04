#[async_trait]
pub trait Provider {

    type Event;

    async fn connect(&mut self) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn append_audio(
        &mut self,
        pcm: &[i16],
    ) -> Result<()>;

    async fn commit(&mut self) -> Result<()>;

    async fn next_event(
        &mut self,
    ) -> Result<Self::Event>;
}
