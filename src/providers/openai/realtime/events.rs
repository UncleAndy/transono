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

    SessionOutputAudioDelta {
        delta: String,
    },

    ResponseOutputAudioDone,

    ResponseDone,

    Error(OpenAiError),

    Unknown,
}
