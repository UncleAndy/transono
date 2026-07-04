use crate::providers::openai::error::OpenAiError;

#[derive(Default)]
pub enum ProtocolEvent {
    SessionOutputAudioDelta {
        delta: String,
    },

    Error(OpenAiError),

    #[default]
    Unknown,
}
