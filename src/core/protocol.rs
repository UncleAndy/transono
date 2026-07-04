pub trait Protocol {
    type Command;
    type Event;

    const ENDPOINT: &'static str;

    fn encode(&self, command: &Self::Command) -> Result<String>;

    fn decode(&self, json: &str) -> Result<Self::Event>;
}
