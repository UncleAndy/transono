use anyhow::anyhow;
use async_trait::async_trait;
use tokio::io;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::audio::{Audio, AudioCodecs, AudioDecoder, AudioEncoder, EncodedAudio, Pipelines};
use crate::core::error::CoreError;
use crate::core::protocol::Protocol;
use crate::core::session::Session;
use crate::core::session_event::SessionEvent;
use crate::core::transport::Transport;
use crate::core::{error::Result, provider::ProviderSession, websocket::WebSocketTransport};
use crate::providers::openai::realtime::{
    AudioConfig, AudioFormat, AudioInputConfig, AudioOutputConfig, InputAudioBufferAppend,
    ProtocolCommand::SessionUpdate, SessionConfig, SessionUpdateEvent, commands::ProtocolCommand,
    config::OpenAIRealtimeConfig, events::ProtocolEvent, protocol::RealtimeProtocol,
};

use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use crate::core::transport::TransportData;

pub struct RealtimeSession {
    closed: bool,

    encoder: Option<Box<dyn AudioEncoder>>,
    decoder: Box<dyn AudioDecoder>,

    transport: WebSocketTransport,
    protocol: RealtimeProtocol,

    config: OpenAIRealtimeConfig,
}

struct RealtimeSender {
    encoder: Box<dyn AudioEncoder>,
    writer_tx: mpsc::Sender<Message>,
    protocol: RealtimeProtocol,
}

impl RealtimeSender {
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

        self.send(InputAudioBufferAppend::new(encoded.bytes().clone()))
            .await?;

        Ok(())
    }
}

impl ProviderSession for RealtimeSession {
    fn spawn(
        mut self,
        mut capture_rx: mpsc::Receiver<Audio>,
        playback_tx: mpsc::Sender<Audio>,
        pipelines: Pipelines,
        cancel: CancellationToken,
        _event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    ) -> JoinHandle<Result<Pipelines>> {
        tokio::spawn(async move {
            let mut stdout = io::stdout();

            let mut jitter_buffer: Vec<Audio> = Vec::new();
            let mut is_playing = false;
            let jitter_threshold = std::time::Duration::from_millis(100);

            // Разделяем пайплайны
            let stats = pipelines.stats.clone();
            let mut input_pipeline = pipelines.input;
            let mut output_pipeline = pipelines.output;

            // Создаем Sender для input_task
            let mut sender = RealtimeSender {
                encoder: self.encoder.take().expect("encoder missing"),
                writer_tx: self.transport.clone_sender(),
                protocol: self.protocol.clone(),
            };

            let cancel_input = cancel.clone();
            let stats_input = stats.clone();
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
                                                stats_input.inc_dropped_network();
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

            'main_loop: loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break 'main_loop;
                    }

                    event = self.next_event() => {
                        let event = match event {
                            Ok(e) => e,
                            Err(e) => {
                                stats.inc_dropped_network();
                                eprintln!("Transport error: {}", e);
                                break;
                            }
                        };
                        match event {
                            SessionEvent::SessionStarted(_) => {
                                // println!("{}", msg);
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
                                                output: AudioOutputConfig {
                                                    format: Some(AudioFormat::pcm_24khz()),
                                                    voice: self.config.voice.clone(),
                                                },
                                            },
                                            output_modalities: None,
                                        },
                                    }
                                )).await?;
                            }
                            SessionEvent::SessionConfigured(_) => {
                                // println!("{}", msg)
                            }
                            SessionEvent::Audio(audio) => {
                                let (audio, pipeline_duration) = output_pipeline.process(audio)?;

                                let total_latency = audio.capture_timestamp().elapsed();

                                if total_latency > std::time::Duration::from_millis(500) {
                                     eprintln!(
                                        "High E2E latency: total={:?}, pipeline={:?}",
                                        total_latency,
                                        pipeline_duration
                                    );
                                }

                                if !is_playing {
                                    jitter_buffer.push(audio);
                                    let buffered_duration: std::time::Duration = jitter_buffer.iter().map(|a| a.duration()).sum();
                                    if buffered_duration >= jitter_threshold {
                                        is_playing = true;
                                        stats.set_output_active(true);
                                        for a in jitter_buffer.drain(..) {
                                            tokio::select! {
                                                _ = cancel.cancelled() => break 'main_loop,
                                                res = playback_tx.send(a) => {
                                                    res.map_err(|_| CoreError::Other(anyhow!("playback channel closed")))?;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    tokio::select! {
                                        _ = cancel.cancelled() => break 'main_loop,
                                        res = playback_tx.send(audio) => {
                                            res.map_err(|_| CoreError::Other(anyhow!("playback channel closed")))?;
                                        }
                                    }
                                }
                            }

                            SessionEvent::Text(delta) => {
                                stdout.write_all(delta.as_bytes()).await
                                    .map_err(|e| CoreError::Other(anyhow!(e)))?;
                                stdout.flush().await
                                    .map_err(|e| CoreError::Other(anyhow!(e)))?;
                            }

                            SessionEvent::RequestStarted => {}

                            SessionEvent::RequestFinished => {}

                            SessionEvent::ResponseStarted => {
                                is_playing = false;
                                jitter_buffer.clear();
                            }

                            SessionEvent::ResponseFinished => {
                                if !is_playing && !jitter_buffer.is_empty() {
                                    stats.set_output_active(true);
                                }
                                for a in jitter_buffer.drain(..) {
                                    tokio::select! {
                                        _ = cancel.cancelled() => break 'main_loop,
                                        res = playback_tx.send(a) => {
                                            res.map_err(|_| CoreError::Other(anyhow!("playback channel closed")))?;
                                        }
                                    }
                                }
                                is_playing = false;
                                stats.set_output_active(false);
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

#[async_trait]
impl Session for RealtimeSession {
    async fn send_audio(&mut self, audio: Audio) -> Result<()> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")));
        }

        let encoder = self.encoder.as_mut().ok_or_else(|| CoreError::Other(anyhow!("encoder taken")))?;

        let pcm = audio.to_pcm()?;

        let encoded = encoder.encode(&pcm)?;

        self.send(InputAudioBufferAppend::new(encoded.bytes().clone()))
            .await?;

        Ok(())
    }

    async fn next_event(&mut self) -> Result<SessionEvent> {
        if self.closed {
            return Err(CoreError::Other(anyhow::Error::msg("session closed")));
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
            return Err(CoreError::Other(anyhow::Error::msg("session closed")));
        }

        self.closed = true;

        let _ = self.transport.close().await;
        Ok(())
    }
}

impl RealtimeSession {
    pub async fn connect(config: &OpenAIRealtimeConfig) -> Result<Self> {
        let request = config.request()?;

        let transport = WebSocketTransport::connect(request).await?;

        let format = config.audio_format();

        Ok(Self {
            closed: false,
            transport,
            protocol: RealtimeProtocol::new(),
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
            ProtocolEvent::ResponseOutputAudioDelta { delta } => Ok(Some(self.map_audio(delta)?)),
            ProtocolEvent::ResponseOutputAudioDone => Ok(None),
            ProtocolEvent::ResponseDone => Ok(Some(SessionEvent::ResponseFinished)),
            ProtocolEvent::InputAudioBufferSpeechStarted => Ok(Some(SessionEvent::RequestStarted)),
            ProtocolEvent::InputAudioBufferSpeechStopped => Ok(Some(SessionEvent::RequestFinished)),
            ProtocolEvent::InputAudioBufferCommitted => Ok(None),
            ProtocolEvent::ResponseCreated => Ok(Some(SessionEvent::ResponseStarted)),
            ProtocolEvent::Error { .. } => Ok(None),
            ProtocolEvent::Unknown => Ok(None),
        }
    }
}
