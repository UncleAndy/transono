use crate::openai::protocol::SessionUpdate;

pub enum ProtocolCommand<'a> {
    SessionUpdate(SessionUpdate),

    InputAudioBufferAppend {
        audio: &'a str,
    },

    InputAudioBufferCommit,

    ResponseCreate(ResponseCreate),

    ResponseCancel,
}
