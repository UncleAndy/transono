pub enum ProtocolEvent {
    SessionOutputAudioDelta {
        delta: String,
    },

    Error(OpenAiError),

    #[default]
    Unknown,
}
