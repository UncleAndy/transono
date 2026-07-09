use serde::Deserialize;
use crate::providers::openai::error::OpenAiError;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolEvent {
    #[serde(rename = "session.created")]
    SessionCreated {
        session: SessionInfo,
    },

    #[serde(rename = "session.updated")]
    SessionUpdated {
        session: SessionInfo,
    },

    #[serde(rename = "session.output_audio.delta")]
    SessionOutputAudioDelta {
        delta: String,
    },

    #[serde(rename = "error")]
    Error(OpenAiError),

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub id: String,

    #[serde(default)]
    pub model: Option<String>,
}
