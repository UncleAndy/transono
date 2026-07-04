pub trait Protocol {
    type ClientEvent;
    type ServerEvent;

    fn endpoint(&self) -> &'static str;

    fn session(&self, cfg: &SessionConfig) -> Self::ClientEvent;

    fn append_audio<'a>(&self, audio: &'a str)
                        -> Self::ClientEvent;

    fn map_event(
        &self,
        event: Self::ServerEvent,
    ) -> Option<RealtimeEvent>;
}
