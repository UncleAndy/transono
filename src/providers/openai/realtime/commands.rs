pub enum ProtocolCommand<'a> {
    SessionUpdate(SessionUpdate),

    InputAudioAppend {
        audio: &'a str,
    },

    InputAudioCommit,

    ResponseCreate(ResponseCreate),

    ResponseCancel,
}
