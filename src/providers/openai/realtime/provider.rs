use crate::core::{
    provider::Provider,
    error::Result,
};
use crate::providers::openai::realtime::{
    session::RealtimeSession,
    config::OpenAIRealtimeConfig,
};

pub struct OpenAIRealtimeProvider {
    config: OpenAIRealtimeConfig,
}

impl OpenAIRealtimeProvider {
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
}
