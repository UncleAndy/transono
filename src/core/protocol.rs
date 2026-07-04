pub trait Protocol {
    type Command;
    type Event;

    fn endpoint(&self) -> &'static str;

    fn encode(
        &self,
        command: &Self::Command,
    ) -> anyhow::Result<String>;

    fn decode(
        &self,
        json: &str,
    ) -> anyhow::Result<Self::Event>;
}
