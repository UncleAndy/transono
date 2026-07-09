use serde::Serialize;

use crate::providers::openai::translation::{SessionConfig};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProtocolCommand {
    SessionUpdate(TranslationSessionUpdate),

    SessionInputAudioBufferAppend(SessionAudioBufferAppend),
}

#[derive(Debug, Serialize)]
pub struct TranslationSessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: SessionConfig,
}

impl TranslationSessionUpdate {
    pub fn new(
        session: SessionConfig,
    ) -> ProtocolCommand {
        ProtocolCommand::SessionUpdate(TranslationSessionUpdate {
            event_type: "session.update",
            session,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct SessionAudioBufferAppend {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub audio: String,
}
