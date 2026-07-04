use crate::audio::frame::AudioFrame;

pub enum ProviderCommand {
    AppendAudio(AudioFrame),
    Commit,
    Cancel,
}
