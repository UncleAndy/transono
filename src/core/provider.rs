use async_trait::async_trait;
// use crate::core::client::Client;
use crate::core::error::Result;

#[async_trait]
pub trait Provider {
    type Error;

    // async fn connect(&self) -> Result<Client<>>;
}
