use async_trait::async_trait;

use crate::core::error::Result;
use crate::core::session::Session;

pub trait ProviderSession {}

#[async_trait]
pub trait Provider {
    type Session: Session + Send + 'static;

    async fn create_session(
        &self,
    ) -> Result<Self::Session>;
}
