use async_trait::async_trait;

use crate::core::provider_event::ProviderEvent;
use crate::core::provider_command::ProviderCommand;
use crate::core::error::Result;

#[async_trait]
pub trait Provider {

    async fn connect(&mut self) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    async fn send(
        &mut self,
        command: ProviderCommand,
    ) -> Result<()>;

    async fn next_event(
        &mut self,
    ) -> Result<ProviderEvent>;
}
