use crate::audio::frame::AudioFrame;

pub enum ProviderEvent {
    Connected,
    Disconnected,
    SpeechStarted,
    SpeechStopped,
    Audio(AudioFrame),
    ResponseFinished,
    Error(anyhow::Error),
}
