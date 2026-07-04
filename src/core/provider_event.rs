pub enum ProviderEvent {
    Connected,
    Disconnected,
    SpeechStarted,
    SpeechStopped,
    Audio(Vec<i16>),
    Finished,
    Error(anyhow::Error),
}
