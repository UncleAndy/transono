use serde::{*};

use crate::providers::openai::realtime::protocol::SessionUpdate;

pub enum ProtocolCommand<'a> {
    SessionUpdate(SessionUpdate),

    InputAudioBufferAppend {
        audio: &'a str,
    },

    InputAudioBufferCommit,

    ResponseCreate(ResponseCreate),

    ResponseCancel,
}

#[derive(Debug, Serialize)]
pub struct ResponseCreate {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCreate {
    pub fn new() -> Self {
        Self {
            event_type: "response.create",
        }
    }
}
