use crate::providers::openai::translation::protocol::{Session, SessionUpdate};

pub enum ProtocolCommand<'a> {
    SessionUpdate(SessionUpdate),

    SessionInputAudioBufferAppend {
        audio: &'a str,
    },

    SessionFinish(Session),
}
