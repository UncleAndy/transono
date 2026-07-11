use serde::Serialize;
use bytes::Bytes;
use crate::core::transport::serialize_bytes_as_str;

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

#[derive(Debug, Serialize)]
pub struct InputAudioTranscription {
    pub model: String
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

    #[serde(serialize_with = "serialize_bytes_as_str")]
    pub audio: Bytes,
}
