//! Low-level bidirectional session contract (audio in, events out).
//!
//! Prefer [`crate::core::provider::Provider`] /
//! [`crate::core::provider::ProviderSession`] for line-level orchestration;
//! this trait models a simpler push/pull session.

use async_trait::async_trait;

use crate::audio::Audio;
use crate::core::error::Result;
use crate::core::session_event::SessionEvent;

/// Bidirectional realtime session: send audio, receive [`SessionEvent`]s.
#[async_trait]
pub trait Session: Send {
    /// Push an audio buffer toward the remote backend.
    ///
    /// # Errors
    ///
    /// Returns transport, protocol, or processing failures from the session.
    async fn send_audio(
        &mut self,
        audio: Audio,
    ) -> Result<()>;

    /// Wait for the next session event.
    ///
    /// # Errors
    ///
    /// Returns transport or protocol failures if the session cannot deliver
    /// the next event.
    async fn next_event(
        &mut self,
    ) -> Result<SessionEvent>;

    /// Close the session and release remote resources.
    ///
    /// # Errors
    ///
    /// Returns transport failures if a clean close cannot be completed.
    async fn close(
        &mut self,
    ) -> Result<()>;
}
