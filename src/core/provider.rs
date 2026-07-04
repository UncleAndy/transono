use std::result::Result;
use async_trait::async_trait;

pub trait ProviderSession: Send {}

#[async_trait]
pub trait Provider {
    type Session: ProviderSession;
    type Error;

    async fn create_session(
        &self,
    ) -> Result<Self::Session, Self::Error>;
}
