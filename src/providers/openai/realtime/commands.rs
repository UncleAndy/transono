use crate::openai::protocol::{ResponseCreate, SessionUpdate};

pub enum ProtocolCommand<'a> {
    SessionUpdate(SessionUpdate),

    InputAudioBufferAppend {
        audio: &'a str,
    },

    InputAudioBufferCommit,

    ResponseCreate(ResponseCreate),

    ResponseCancel,
}
