use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use cpal::SampleFormat;
use futures_util::SinkExt;
use async_trait::async_trait;

use crate::audio::{Audio, AudioEncoder, BinaryEncoding, AudioFormat, EncodedAudio, Endianness, PcmCodec};
use crate::core::{
    websocket::WebSocketTransport,
    error::Result,
    provider::ProviderSession,
};
use crate::core::error::CoreError;
use crate::core::protocol::Protocol;
use crate::core::session::Session;
use crate::core::session_event::SessionEvent;
use crate::core::transport::Transport;
use crate::providers::openai::realtime::commands::ProtocolCommand;
use crate::providers::openai::realtime::events::ProtocolEvent;
use super::{protocol::RealtimeProtocol, config::OpenAIRealtimeConfig, InputAudioBufferAppend};

pub struct RealtimeSession {
    closed: bool,

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
            closed: false,
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

    async fn send(
        &mut self,
        command: ProtocolCommand,
    ) -> Result<()>
    {
        let data = self.protocol.encode(&command)?;

        self.transport.send(data).await
    }

    fn map_audio(
        &mut self,
        delta: String,
    ) -> Result<SessionEvent> {
        let bytes = BASE64_STANDARD.decode(delta)
            .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

        let encoded = EncodedAudio::new(
            BinaryEncoding::Pcm {
                endianness: Endianness::Little,
            },
            bytes.into(),
        );

        let audio = self.codec.decode(&encoded)?;

        Ok(SessionEvent::Audio(audio))
    }

    fn map_event(
        &mut self,
        event: ProtocolEvent,
    ) -> Result<Option<SessionEvent>> {
        match event {
            ProtocolEvent::SessionCreated { .. } => {
                Ok(None)
            }
            ProtocolEvent::SessionUpdated { .. } => {
                Ok(None)
            }
            ProtocolEvent::ResponseOutputAudioDelta { delta } => {
                Ok(Some(self.map_audio(delta)?))
            }
            ProtocolEvent::ResponseOutputAudioDone => {
                Ok(None)
            }
            ProtocolEvent::ResponseDone => {
                Ok(Some(SessionEvent::ResponseFinished))
            }
            ProtocolEvent::InputAudioBufferSpeechStarted => {
                Ok(Some(SessionEvent::RequestStarted))
            }
            ProtocolEvent::InputAudioBufferSpeechStopped => {
                Ok(Some(SessionEvent::RequestFinished))
            }
            ProtocolEvent::InputAudioBufferCommitted => {
                Ok(None)
            }
            ProtocolEvent::ResponseCreated => {
                Ok(Some(SessionEvent::ResponseStarted))
            }
            ProtocolEvent::Error { .. } => {
                Ok(None)
            }
            ProtocolEvent::Unknown => {
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl Session for RealtimeSession {
    async fn send_audio(
        &mut self,
        audio: Audio,
    ) -> Result<()> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")))
        }

        let encoded =
            self.codec.encode(&audio)?;

        let base64 =
            BASE64_STANDARD.encode(encoded.bytes());

        self.send(
            ProtocolCommand::InputAudioBufferAppend(
                InputAudioBufferAppend {
                    event_type: "input_audio_buffer.append",
                    audio: base64,
                }
            )
        ).await?;

        Ok(())
    }

    async fn next_event(
        &mut self,
    ) -> Result<SessionEvent> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")))
        }

        loop {
            let data = self.transport.recv().await?;

            let event = self.protocol.decode(data)?;

            if let Some(event) = self.map_event(event)? {
                return Ok(event);
            }
        }
    }

    async fn close(&mut self) -> Result<()> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")))
        }

        self.closed = true;

        let _ = self.transport.close().await;
        Ok(())
    }
}
