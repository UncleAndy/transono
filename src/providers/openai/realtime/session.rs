use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc;
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
use crate::providers::openai::realtime::{
    InputAudioBufferAppend,
    SessionUpdateEvent,
    SessionConfig,
    AudioConfig,
    AudioOutputConfig,
    AudioInputConfig,
    AudioFormat,
    ProtocolCommand::SessionUpdate,
    protocol::RealtimeProtocol,
    config::OpenAIRealtimeConfig,
    commands::ProtocolCommand,
    events::ProtocolEvent,
};

pub struct RealtimeSession {
    closed: bool,

    encoder: Box<dyn AudioEncoder>,
    decoder: Box<dyn AudioDecoder>,

    transport: WebSocketTransport,
    protocol: RealtimeProtocol,

    config: OpenAIRealtimeConfig,
}

impl ProviderSession for RealtimeSession {
    fn spawn(
        mut self,
        mut capture_rx: mpsc::Receiver<Audio>,
        playback_tx: mpsc::Sender<Audio>,
        mut pipelines: Pipelines,
        cancel: CancellationToken,
    ) -> JoinHandle<Result<Pipelines>>
    {
        tokio::spawn(async move {
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
                        SessionEvent::SessionStarted(_) => {
                            // Отправляем конфиг в 'session.update'
                            self.send(SessionUpdate(
                                SessionUpdateEvent {
                                    event_type: "session.update",
                                    session: SessionConfig {
                                        session_type: Some("realtime"),
                                        model: self.config.model.clone(),
                                        instructions: self.config.instructions.clone(),
                                        audio: AudioConfig {
                                            input: Some(
                                                    AudioInputConfig {
                                                        format: Some(AudioFormat::pcm_24khz()),
                                                        turn_detection: Some(self.config.turn_mode.clone()),
                                                    }
                                                ),
                                            output: AudioOutputConfig { format: None,voice: None},
                                        },
                                        output_modalities: None,
                                    },
                                }
                            )).await?;
                        }
                        SessionEvent::Audio(audio) => {
                            let audio = pipelines.output.process(audio)?;

                            playback_tx
                                .send(audio)
                                .await
                                .map_err(|_| CoreError::Other(anyhow!("playback channel closed")))?;

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
