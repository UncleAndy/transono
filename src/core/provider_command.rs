//! Outbound commands from the application toward a provider session.

use crate::audio::frame::AudioFrame;

/// Command sent into a provider session's input path.
pub enum ProviderCommand {
    /// Append a captured audio frame to the provider input buffer.
    AppendAudio(AudioFrame),
    /// Commit buffered input so the provider can process it.
    Commit,
    /// Cancel the in-flight request or generation.
    Cancel,
}
