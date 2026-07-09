use serde::Serialize;

use crate::providers::openai::translation::{SessionConfig};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProtocolCommand {
    SessionUpdate(TranslationSessionUpdateEvent),

    SessionInputAudioBufferAppend(SessionAudioBufferAppend),
}

#[derive(Debug, Serialize)]
pub struct TranslationSessionUpdateEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: SessionConfig,
}

impl TranslationSessionUpdateEvent {
    pub fn new(
        session: SessionConfig,
    ) -> ProtocolCommand {
        ProtocolCommand::SessionUpdate(TranslationSessionUpdateEvent {
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
