pub enum ProtocolCommand<'a> {
    SessionUpdate(SessionUpdate),

    SessionInputAudioBufferAppend {
        audio: &'a str,
    },

    SessionFinish(Session),
}
