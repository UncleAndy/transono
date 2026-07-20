//! [`OpenAIRealtimeProvider`] factory for OpenAI Realtime sessions.

use crate::core::{
    provider::Provider,
    error::Result,
};
use crate::audio::EncodedAudioFormat;
use crate::providers::openai::realtime::{
    session::RealtimeSession,
    config::OpenAIRealtimeConfig,
};

/// [`Provider`] factory for OpenAI Realtime sessions.
///
/// Holds an [`OpenAIRealtimeConfig`] and opens a [`RealtimeSession`] over WebSocket.
///
/// # Examples
///
/// ```no_run
/// use transono::core::provider::Provider;
/// use transono::providers::openai::realtime::{
///     OpenAIRealtimeConfig, OpenAIRealtimeProvider,
/// };
///
/// # async fn demo() -> transono::core::error::Result<()> {
/// let provider = OpenAIRealtimeProvider::new(OpenAIRealtimeConfig::from_env()?);
/// let _session = provider.create_session().await?;
/// # Ok(())
/// # }
/// ```
pub struct OpenAIRealtimeProvider {
    config: OpenAIRealtimeConfig,
}

impl OpenAIRealtimeProvider {
    /// Wrap a Realtime config as a provider factory.
    pub fn new(config: OpenAIRealtimeConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Provider for OpenAIRealtimeProvider {
    type Session = RealtimeSession;

    async fn create_session(
        &self,
    ) -> Result<Self::Session> {
        RealtimeSession::connect(&self.config).await
    }

    fn audio_format(&self) -> EncodedAudioFormat {
        self.config.audio_format()
    }
}
