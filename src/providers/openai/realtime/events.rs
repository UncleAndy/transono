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

    #[serde(rename = "response.output_audio.delta")]
    ResponseOutputAudioDelta {
        delta: String,
    },

    #[serde(rename = "response.output_audio.done")]
    ResponseOutputAudioDone,

    #[serde(rename = "response.done")]
    ResponseDone,

    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted,

    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped,

    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted,

    #[serde(rename = "response.created")]
    ResponseCreated,

    #[serde(rename = "error")]
    Error {
        error: OpenAiError,
    },

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    pub id: String,

    #[serde(default)]
    pub model: Option<String>,
}
