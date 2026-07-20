//! Lifecycle states for [`super::TranslationLine`].

/// Lifecycle of a [`crate::line::TranslationLine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineState {
    /// Constructed; not yet running.
    Created,
    /// Capture, playback, and provider session are active.
    Running,
    /// Shutdown in progress (reserved / transitional).
    Stopping,
    /// Fully stopped; may be started again after re-attach of audio.
    Stopped,
}
