//! Inbound events emitted by a provider session toward the application.

use crate::audio::frame::AudioFrame;

/// Event produced by a provider session during its lifetime.
pub enum ProviderEvent {
    /// Transport connection to the remote backend is established.
    Connected,
    /// Transport connection was closed or lost.
    Disconnected,
    /// Remote VAD or similar detected the start of user speech.
    SpeechStarted,
    /// Remote VAD or similar detected the end of user speech.
    SpeechStopped,
    /// Decoded audio frame from the provider (playback path).
    Audio(AudioFrame),
    /// Provider finished generating the current response.
    ResponseFinished,
    /// Unrecoverable or reported error from the provider path.
    Error(anyhow::Error),
}
