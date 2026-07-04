use crate::providers::openai::error::OpenAiError;
use crate::providers::openai::realtime::protocol::Session;

#[derive(Default)]
pub enum ProtocolEvent {
    SessionCreated(Session),

    SessionUpdated(Session),

    InputAudioBufferSpeechStarted,

    InputAudioBufferSpeechStopped,

    InputAudioBufferCommitted,

    ResponseCreated,

    ResponseOutputAudioDelta {
        delta: String,
    },

    ResponseOutputAudioDone,

    ResponseDone,

    Error(OpenAiError),

    #[default]
    Unknown,
}
