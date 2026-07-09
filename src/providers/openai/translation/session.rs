use anyhow::anyhow;
use async_trait::async_trait;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use crate::audio::{Audio, AudioEncoder, EncodedAudio, AudioDecoder, AudioCodecs, Pipelines};
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

use crate::providers::openai::translation::{TranslationProtocol, SessionAudioBufferAppend, config::OpenAITranslationConfig, commands::ProtocolCommand, ProtocolEvent, TranslationSessionUpdateEvent, SessionConfig, AudioConfig, AudioOutputConfig};
use crate::providers::openai::translation::ProtocolCommand::SessionUpdate;

pub struct TranslationSession {
    closed: bool,

    encoder: Box<dyn AudioEncoder>,
    decoder: Box<dyn AudioDecoder>,

    transport: WebSocketTransport,
    protocol: TranslationProtocol,

    config: OpenAITranslationConfig,
}

impl ProviderSession for TranslationSession {
    fn spawn(
        mut self,
        mut capture_rx: Receiver<Audio>,
        playback_tx: Sender<Audio>,
        mut pipelines: Pipelines,
        cancel: CancellationToken,
    ) -> JoinHandle<Result<Pipelines>>
    {
        tokio::spawn(async move {
            let mut stdout = io::stdout();

            loop {
                tokio::select! {
                _ = cancel.cancelled() => {
                    break;
                }

                Some(audio) = capture_rx.recv() => {
                    let audio = pipelines.input.process(audio)?;

                    self.send_audio(audio).await?;
                }

                event = self.next_event() => {
                    match event? {
                        SessionEvent::SessionStarted(msg) => {
                            println!("{}", msg);
                            // Отправляем конфиг в 'session.update'
                            self.send(SessionUpdate(
                                TranslationSessionUpdateEvent {
                                    event_type: "session.update",
                                    session: SessionConfig {
                                        audio: AudioConfig {
                                            input: None,
                                            output: AudioOutputConfig {
                                                format: None,
                                                language: self.config.lang.clone(),
                                            },
                                        },
                                    },
                                }
                            )).await?;
                        }
                        SessionEvent::SessionConfigured(msg) => {
                            println!("{}", msg);
                        }
                        SessionEvent::Audio(audio) => {
                            let audio = pipelines.output.process(audio)?;

                            playback_tx
                                .send(audio)
                                .await
                                .map_err(|_| CoreError::Other(anyhow!("playback channel closed")))?;

                        }

                        SessionEvent::Text(delta) => {
                            stdout.write_all(delta.as_bytes()).await
                                .map_err(|e| CoreError::Other(anyhow!(e)))?;
                            stdout.flush().await
                                .map_err(|e| CoreError::Other(anyhow!(e)))?;
                        }

                        SessionEvent::RequestStarted => {}

                        SessionEvent::RequestFinished => {}

                        SessionEvent::ResponseStarted => {}

                        SessionEvent::ResponseFinished => {}
                    }
                }
            }
            }

            self.close().await?;

            Ok(pipelines)
        })
    }
}

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
            config: config.clone(),
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
                Ok(Some(SessionEvent::SessionStarted("Translation session created".to_string())))
            }
            ProtocolEvent::SessionUpdated { .. } => {
                Ok(Some(SessionEvent::SessionConfigured("Translation session configured".to_string())))
            }
            ProtocolEvent::SessionOutputAudioDelta { delta } => {
                Ok(Some(self.map_audio(delta)?))
            }
            ProtocolEvent::SessionOutputTranscriptDelta { delta } => {
                Ok(Some(SessionEvent::Text(delta)))
            }
            ProtocolEvent::Error(e) => {
                println!("Error: {}", e);
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

            let event = self.protocol.decode(data.clone())?;

            if let Some(event) = self.map_event(event)? {
                return Ok(event);
            } else {
                println!("ERROR MAP EVENT: {:#?}", data);
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
