pub enum ProviderEvent {
    Connected,
    Disconnected,
    SpeechStarted,
    SpeechStopped,
    Audio(AudioFrame<'a>),
    ResponseFinished,
    Error(anyhow::Error),
}
