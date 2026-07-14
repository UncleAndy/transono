use crate::core::{provider::Provider, error::Result};
use crate::audio::EncodedAudioFormat;
use crate::providers::openai::translation::{OpenAITranslationConfig, TranslationSession};

pub struct OpenAITranslationProvider {
    config: OpenAITranslationConfig,
}

impl OpenAITranslationProvider {
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
