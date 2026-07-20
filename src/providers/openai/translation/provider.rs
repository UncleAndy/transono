//! [`OpenAITranslationProvider`] factory for OpenAI speech-translation sessions.

use crate::core::{provider::Provider, error::Result};
use crate::audio::EncodedAudioFormat;
use crate::providers::openai::translation::{OpenAITranslationConfig, TranslationSession};

/// [`Provider`] factory for OpenAI speech-translation sessions.
///
/// Holds an [`OpenAITranslationConfig`] and opens a [`TranslationSession`] over
/// WebSocket. This is the backend used by the `transono` binary for capture →
/// translate → playback lines.
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
/// let mut cfg = OpenAITranslationConfig::from_env()?;
/// cfg.with_lang("en");
/// let provider = OpenAITranslationProvider::new(cfg);
/// let _session = provider.create_session().await?;
/// # Ok(())
/// # }
/// ```
pub struct OpenAITranslationProvider {
    config: OpenAITranslationConfig,
}

impl OpenAITranslationProvider {
    /// Wrap a Translation config as a provider factory.
    pub fn new(config: OpenAITranslationConfig) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl Provider for OpenAITranslationProvider {
    type Session = TranslationSession;

    async fn create_session(
        &self,
    ) -> Result<Self::Session> {
        TranslationSession::connect(&self.config).await
    }

    fn audio_format(&self) -> EncodedAudioFormat {
        self.config.audio_format()
    }
}
