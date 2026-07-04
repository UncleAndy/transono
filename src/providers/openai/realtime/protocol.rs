#[derive(Default)]
pub struct RealtimeProtocol;

impl Protocol for RealtimeProtocol {
    type Command = ProtocolCommand<'static>;
    type Event = ProtocolEvent;

    const ENDPOINT: &'static str = "/v1/realtime";
}
