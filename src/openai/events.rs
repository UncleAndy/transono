use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "session.created")]
    SessionCreated { session: Session },

    #[serde(rename = "session.updated")]
    SessionUpdated { session: Session },

    #[serde(rename = "response.output_audio.delta")]
    ResponseOutputAudioDelta { delta: String },

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
    Error { error: ErrorInfo },

    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,

    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorInfo {
    pub message: String,

    #[serde(default)]
    pub code: Option<String>,
}
