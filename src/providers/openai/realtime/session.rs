use crate::audio::{Audio, AudioCodec};
use crate::core::{
    websocket::WebSocketTransport,
    error::Result,
    provider::ProviderSession,
};
use crate::core::protocol::Protocol;
use crate::core::session::Session;
use crate::core::transport::Transport;
use crate::providers::openai::realtime::commands::ProtocolCommand;
use crate::providers::openai::realtime::events::ProtocolEvent;
use super::{
    protocol::RealtimeProtocol,
    config::OpenAIRealtimeConfig,
};

pub struct RealtimeSession {
    pub codec: PcmCodec,

    transport: WebSocketTransport,
    protocol: RealtimeProtocol,
}

impl ProviderSession for RealtimeSession {}

impl RealtimeSession {
    pub async fn connect(
        config: &OpenAIRealtimeConfig,
    ) -> Result<Self> {
        let request = config.request()?;

        let transport =
            WebSocketTransport::connect(request)
                .await?;

        Ok(Self {
            transport,
            protocol: RealtimeProtocol::new(),
            codec: PcmCodec::new(
                AudioFormat {
                    sample_rate: 24_000,
                    channels: 1,
                    sample_format: SampleFormat::I16,
                },
                Endianness::Little,
            ),
        })
    }
}

impl Session for RealtimeSession {
    async fn send_audio(
        &mut self,
        audio: Audio,
    ) -> Result<()> {

        let encoded =
            self.codec.encode(audio)?;

        let base64 =
            BASE64_STANDARD.encode(encoded.data());

        self.send(
            InputAudioBufferAppend {
                audio: base64,
            }
        ).await?;

        Ok(())
    }

    pub async fn next_event(
        &mut self,
    ) -> Result<ProtocolEvent> {

        let data = self.transport.recv().await?;

        self.protocol.decode(data)
    }
}
