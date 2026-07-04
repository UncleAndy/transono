use std::result::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Provider {
    type Session;
    type Error;

    async fn create_session(
        &self,
    ) -> Result<Self::Session, Self::Error>;
}
