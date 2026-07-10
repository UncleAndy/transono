use anyhow::anyhow;
use async_trait::async_trait;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::audio::{Audio, AudioCodecs, AudioDecoder, AudioEncoder, EncodedAudio, Pipelines};
use crate::core::error::CoreError;
use crate::core::protocol::Protocol;
use crate::core::session::Session;
use crate::core::session_event::SessionEvent;
use crate::core::transport::Transport;
use crate::core::{error::Result, provider::ProviderSession, websocket::WebSocketTransport};

use crate::providers::openai::translation::ProtocolCommand::SessionUpdate;
use crate::providers::openai::translation::{
    AudioConfig, AudioOutputConfig, ProtocolEvent, SessionAudioBufferAppend, SessionConfig,
    TranslationProtocol, TranslationSessionUpdateEvent, commands::ProtocolCommand,
    config::OpenAITranslationConfig,
};

use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use crate::core::transport::TransportData;

pub struct TranslationSession {
    closed: bool,

    encoder: Option<Box<dyn AudioEncoder>>,
    decoder: Box<dyn AudioDecoder>,

    transport: WebSocketTransport,
    protocol: TranslationProtocol,

    config: OpenAITranslationConfig,
}

struct TranslationSender {
    encoder: Box<dyn AudioEncoder>,
    writer_tx: mpsc::Sender<Message>,
    protocol: TranslationProtocol,
}

impl TranslationSender {
    async fn send(&mut self, command: ProtocolCommand) -> Result<()> {
        let data = self.protocol.encode(&command)?;
        let message = match data {
            TransportData::Text(data) => {
                Message::Text(unsafe { Utf8Bytes::from_bytes_unchecked(data) })
            }
            TransportData::Binary(data) => Message::Binary(data),
        };
        self.writer_tx.send(message).await.map_err(|_| CoreError::Transport(crate::core::error::TransportError::ConnectionClosed))?;
        Ok(())
    }

    async fn send_audio(&mut self, audio: Audio) -> Result<()> {
        let pcm = audio.to_pcm()?;
        let encoded = self.encoder.encode(&pcm)?;

        self.send(ProtocolCommand::SessionInputAudioBufferAppend(
            SessionAudioBufferAppend {
                event_type: "session.input_audio_buffer.append",
                audio: encoded.bytes().clone(),
            },
        ))
        .await?;

        Ok(())
    }
}

impl ProviderSession for TranslationSession {
    fn spawn(
        mut self,
        mut capture_rx: Receiver<Audio>,
        playback_tx: Sender<Audio>,
        pipelines: Pipelines,
        cancel: CancellationToken,
        event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    ) -> JoinHandle<Result<Pipelines>> {
        tokio::spawn(async move {
            // Разделяем пайплайны
            let stats = pipelines.stats.clone();
            let mut input_pipeline = pipelines.input;
            let mut output_pipeline = pipelines.output;

            // Создаем Sender для input_task
            let mut sender = TranslationSender {
                encoder: self.encoder.take().expect("encoder missing"),
                writer_tx: self.transport.clone_sender(),
                protocol: self.protocol.clone(),
            };

            let cancel_input = cancel.clone();
            let input_task = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel_input.cancelled() => break,
                        audio = capture_rx.recv() => {
                            match audio {
                                Some(audio) => {
                                    let capture_ts = audio.capture_timestamp();
                                    let (audio, pipeline_duration) = input_pipeline.process(audio)?;

                                    let processing_latency = capture_ts.elapsed();
                                    if processing_latency > std::time::Duration::from_millis(100) {
                                        eprintln!(
                                            "High input latency: total={:?}, pipeline={:?}",
                                            processing_latency,
                                            pipeline_duration
                                        );
                                    }

                                    // Отправляем аудио напрямую из input_task
                                    tokio::select! {
                                        _ = cancel_input.cancelled() => break,
                                        res = sender.send_audio(audio) => {
                                            if let Err(e) = res {
                                                eprintln!("Error sending audio: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                }
                                None => break,
                            }
                        }
                    }
                }
                Ok(input_pipeline)
            });

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }

                    event = self.next_event() => {
                        match event? {
                            SessionEvent::SessionStarted(_) => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::SessionStarted("Translation session started".to_string()));
                                }
                                // println!("{}", msg);
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
                            SessionEvent::SessionConfigured(_) => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::SessionConfigured("Translation session configured".to_string()));
                                }
                            }
                            SessionEvent::Audio(audio) => {
                                let (audio, pipeline_duration) = output_pipeline.process(audio)?;

                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::Audio(audio.clone()));
                                }

                                let total_latency = audio.capture_timestamp().elapsed();

                                if total_latency > std::time::Duration::from_millis(500) {
                                     eprintln!(
                                        "High E2E latency: total={:?}, pipeline={:?}",
                                        total_latency,
                                        pipeline_duration
                                    );
                                }

                                tokio::select! {
                                    _ = cancel.cancelled() => break,
                                    res = playback_tx.send(audio) => {
                                        res.map_err(|_| CoreError::Other(anyhow!("playback channel closed")))?;
                                    }
                                }

                            }

                            SessionEvent::Text(delta) => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::Text(delta));
                                }
                            }

                            SessionEvent::RequestStarted => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::RequestStarted);
                                }
                            }

                            SessionEvent::RequestFinished => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::RequestFinished);
                                }
                            }

                            SessionEvent::ResponseStarted => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::ResponseStarted);
                                }
                            }

                            SessionEvent::ResponseFinished => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::ResponseFinished);
                                }
                            }
                        }
                    }
                }
            }

            self.close().await?;

            // Собираем пайплайны обратно
            let input_pipeline = input_task.await
                .map_err(|e| CoreError::Other(anyhow!("input task panicked: {}", e)))?
                .map_err(|e: CoreError| CoreError::Other(anyhow!("input task error: {}", e)))?;

            Ok(Pipelines {
                input: input_pipeline,
                output: output_pipeline,
                stats,
            })
        })
    }
}

impl TranslationSession {
    pub async fn connect(config: &OpenAITranslationConfig) -> Result<Self> {
        let request = config.request()?;

        let transport = WebSocketTransport::connect(request).await?;

        let format = config.audio_format();

        Ok(Self {
            closed: false,
            transport,
            protocol: TranslationProtocol::new(),
            encoder: Some(AudioCodecs::encoder(&format)?),
            decoder: AudioCodecs::decoder(&format)?,
            config: config.clone(),
        })
    }

    async fn send(&mut self, command: ProtocolCommand) -> Result<()> {
        let data = self.protocol.encode(&command)?;

        self.transport.send(data).await
    }

    fn map_audio(&mut self, delta: String) -> Result<SessionEvent> {
        let encoded = EncodedAudio::new(self.decoder.format().clone(), delta.into_bytes().into());

        let pcm = self.decoder.decode(&encoded)?;

        let audio = Audio::from_pcm(&pcm)?;

        Ok(SessionEvent::Audio(audio))
    }

    fn map_event(&mut self, event: ProtocolEvent) -> Result<Option<SessionEvent>> {
        match event {
            ProtocolEvent::SessionCreated { .. } => Ok(Some(SessionEvent::SessionStarted(
                "Translation session created".to_string(),
            ))),
            ProtocolEvent::SessionUpdated { .. } => Ok(Some(SessionEvent::SessionConfigured(
                "Translation session configured".to_string(),
            ))),
            ProtocolEvent::SessionOutputAudioDelta { delta } => Ok(Some(self.map_audio(delta)?)),
            ProtocolEvent::SessionOutputTranscriptDelta { delta } => {
                Ok(Some(SessionEvent::Text(delta)))
            }
            ProtocolEvent::Error(e) => {
                println!("Error: {}", e);
                Err(CoreError::Other(anyhow!(e)))
            }
            ProtocolEvent::Unknown => Ok(None),
        }
    }
}

#[async_trait]
impl Session for TranslationSession {
    async fn send_audio(&mut self, audio: Audio) -> Result<()> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")));
        }

        let encoder = self.encoder.as_mut().ok_or_else(|| CoreError::Other(anyhow!("encoder taken")))?;

        let pcm = audio.to_pcm()?;

        let encoded = encoder.encode(&pcm)?;

        self.send(ProtocolCommand::SessionInputAudioBufferAppend(
            SessionAudioBufferAppend {
                event_type: "session.input_audio_buffer.append",
                audio: encoded.bytes().clone(),
            },
        ))
        .await?;

        Ok(())
    }

    async fn next_event(&mut self) -> Result<SessionEvent> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")));
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
            return Err(CoreError::Other(anyhow::Error::msg("session closed")));
        }

        self.closed = true;

        let _ = self.transport.close().await;
        Ok(())
    }
}
