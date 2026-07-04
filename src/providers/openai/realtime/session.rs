use crate::core::{
    websocket::WebSocketTransport,
    error::Result,
    provider::ProviderSession,
};
use crate::core::protocol::Protocol;
use crate::core::transport::Transport;
use crate::providers::openai::realtime::commands::ProtocolCommand;
use crate::providers::openai::realtime::events::ProtocolEvent;
use super::{
    protocol::RealtimeProtocol,
    config::OpenAIRealtimeConfig,
};

pub struct RealtimeSession {
    transport: WebSocketTransport,
    protocol: RealtimeProtocol,
}

impl ProviderSession for RealtimeSession {}

impl RealtimeSession {
    pub async fn connect(
        config: &OpenAIRealtimeConfig,
    ) -> Result<Self> {

        let transport =
            WebSocketTransport::connect(
                config.request()?,
            )
                .await?;

        Ok(Self {
            transport,
            protocol: RealtimeProtocol::new(),
        })
    }

    pub async fn send(
        &mut self,
        command: ProtocolCommand,
    ) -> Result<()> {

        let data = self.protocol.encode(&command)?;

        self.transport.send(data).await?;

        Ok(())
    }

    pub async fn next_event(
        &mut self,
    ) -> Result<ProtocolEvent> {

        let data = self.transport.recv().await?;

        self.protocol.decode(data)
    }
}
