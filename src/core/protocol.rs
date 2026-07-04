pub trait Protocol: Send + Sync + 'static {
    type Command;
    type Event;

    const ENDPOINT: &'static str;

    fn encode(
        &self,
        command: &Self::Command,
    ) -> Result<Vec<u8>>;

    fn decode(
        &self,
        data: &[u8],
    ) -> Result<Self::Event>;
}
