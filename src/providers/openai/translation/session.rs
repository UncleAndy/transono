use anyhow::anyhow;
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
use crate::providers::openai::translation::{
    TranslationProtocol,
    SessionAudioBufferAppend,
    config::OpenAITranslationConfig,
    commands::ProtocolCommand,
    events::ProtocolEvent,
};

pub struct TranslationSession {
    closed: bool,

    encoder: Box<dyn AudioEncoder>,
    decoder: Box<dyn AudioDecoder>,

    transport: WebSocketTransport,
    protocol: TranslationProtocol,
}

impl ProviderSession for TranslationSession {}

impl TranslationSession {
    pub async fn connect(
        config: &OpenAITranslationConfig,
    ) -> Result<Self> {
        let request = config.request()?;

        let transport =
            WebSocketTransport::connect(request)
                .await?;

        let format = config.audio_format();

        Ok(Self {
            closed: false,
            transport,
            protocol: TranslationProtocol::new(),
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
            ProtocolEvent::SessionUpdated { .. } => {
                Ok(None)
            }
            ProtocolEvent::SessionOutputAudioDelta { delta } => {
                Ok(Some(self.map_audio(delta)?))
            }
            ProtocolEvent::Error(e) => {
                Err(CoreError::Other(anyhow!(e)))
            }
            ProtocolEvent::Unknown => {
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl Session for TranslationSession {
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
            ProtocolCommand::SessionInputAudioBufferAppend(
                SessionAudioBufferAppend {
                    event_type: "session.input_audio_buffer.append",
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
