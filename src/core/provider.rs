//! AI provider factory and session spawning contracts.
//!
//! [`Provider`] is independent of any concrete transport or wire format;
//! implementations choose how to reach the remote backend.

use async_trait::async_trait;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio::sync::mpsc;
use futures_util::stream::BoxStream;
use crate::audio::output::BoxSink;
use crate::audio::{Audio, Pipelines, EncodedAudioFormat};
use crate::core::error::{Result, CoreError};
use crate::core::session_event::SessionEvent;

/// Running provider session that consumes capture and drives playback.
///
/// Implementations typically own a WebSocket (or similar) connection and
/// bridge encoded audio in both directions until `cancel` fires.
pub trait ProviderSession {
    /// Spawn the session on the Tokio runtime.
    ///
    /// Returns a join handle that yields the pipelines when the session ends
    /// so the caller can reclaim DSP state after stop.
    ///
    /// # Errors
    ///
    /// The join handle resolves to [`CoreError`] if the session fails
    /// (transport, protocol, or internal processing).
    fn spawn(
        self,
        capture_stream: BoxStream<'static, Audio>,
        playback_sink: BoxSink<'static, Audio, CoreError>,
        pipelines: Pipelines,
        cancel: CancellationToken,
        event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    ) -> JoinHandle<Result<Pipelines>>;
}

/// Factory for provider sessions and the audio format they expect.
///
/// Application code depends on this trait rather than a specific AI vendor.
/// Concrete backends live under [`crate::providers`].
///
/// # Examples
///
/// ```no_run
/// use transono::core::provider::Provider;
/// use transono::providers::openai::translation::{
///     OpenAITranslationConfig, OpenAITranslationProvider,
/// };
///
/// # async fn demo() -> transono::core::error::Result<()> {
/// let provider = OpenAITranslationProvider::new(OpenAITranslationConfig::from_env()?);
/// let _session = provider.create_session().await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait Provider {
    /// Session type produced by this provider.
    type Session: ProviderSession;

    /// Open a new realtime session with the remote backend.
    ///
    /// # Errors
    ///
    /// Returns transport/protocol errors if the connection or handshake fails.
    async fn create_session(&self) -> Result<Self::Session>;

    /// Encoded audio format required by the remote session.
    fn audio_format(&self) -> EncodedAudioFormat;
}
