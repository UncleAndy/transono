use async_trait::async_trait;

use crate::audio::{
    Audio,
    AudioEncoder,
    EncodedAudio,
    AudioDecoder,
    AudioCodecs,
};
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
use crate::providers::openai::realtime::{
    protocol::RealtimeProtocol,
    config::OpenAIRealtimeConfig,
    InputAudioBufferAppend,
    commands::ProtocolCommand,
    events::ProtocolEvent,
};

pub struct RealtimeSession {
    closed: bool,

    encoder: Box<dyn AudioEncoder>,
    decoder: Box<dyn AudioDecoder>,

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

        let format = config.audio_format();

        Ok(Self {
            closed: false,
            transport,
            protocol: RealtimeProtocol::new(),
            encoder: AudioCodecs::encoder(&format)?,
            decoder: AudioCodecs::decoder(&format)?,
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
        let encoded = EncodedAudio::new(
            self.decoder.format().clone(),
            delta.into_bytes().into(),
        );

        let pcm =
            self.decoder.decode(&encoded)?;

        let audio =
            Audio::from_pcm(&pcm)?;

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

        let pcm = audio.to_pcm()?;

        let encoded = self.encoder.encode(&pcm)?;

        self.send(
            ProtocolCommand::InputAudioBufferAppend(
                InputAudioBufferAppend {
                    event_type: "input_audio_buffer.append",
                    audio: String::from_utf8(
                        encoded.bytes().to_vec(),
                    )
                        .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?,
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
