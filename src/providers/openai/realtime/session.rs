use crate::core::{
    websocket::WebSocketTransport,
    error::{ CoreError, Result },
    provider::ProviderSession,
};

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
}
