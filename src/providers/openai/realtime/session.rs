//! [`RealtimeSession`] — live OpenAI Realtime WebSocket session.
//!
//! Like the Translation session, a Realtime session can emit both
//! transcribed input text (`SessionEvent::InputText`) and model response
//! text (`SessionEvent::Text`) when `input_audio_transcription` is enabled
//! on [`OpenAIRealtimeConfig`] and `Text` modality is requested.

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::{SinkExt, StreamExt};
use crate::audio::output::BoxSink;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::audio::{Audio, AudioCodecs, AudioDecoder, AudioEncoder, EncodedAudio, Pipelines, Pipeline};
use crate::core::error::CoreError;
use crate::core::protocol::Protocol;
use crate::core::session::Session;
use crate::core::session_event::SessionEvent;
use crate::core::transport::Transport;
use crate::core::{error::Result, provider::ProviderSession, websocket::WebSocketTransport};
use crate::providers::openai::realtime::{
    AudioConfig, AudioFormat, AudioInputConfig, AudioOutputConfig, InputAudioBufferAppend,
    InputAudioTranscription, OutputModality, ProtocolCommand::SessionUpdate,
    SessionConfig, SessionUpdateEvent, commands::ProtocolCommand, config::OpenAIRealtimeConfig,
    events::ProtocolEvent, protocol::RealtimeProtocol,
};

use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use crate::core::transport::TransportData;

/// Live OpenAI Realtime WebSocket session.
///
/// Implements [`ProviderSession`] for line-level capture→playback bridging and
/// [`Session`] for lower-level push/pull use. After [`Self::connect`], the first
/// server `session.created` triggers a `session.update` with audio, turn
/// detection, and optional input transcription settings from
/// [`OpenAIRealtimeConfig`].
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
        mut capture_stream: BoxStream<'static, Audio>,
        mut playback_sink: BoxSink<'static, Audio, CoreError>,
        pipelines: Pipelines,
        cancel: CancellationToken,
        event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    ) -> JoinHandle<Result<Pipelines>> {
        tokio::spawn(async move {
            let mut jitter_buffer: Vec<Audio> = Vec::new();
            let mut is_playing = false;
            let jitter_threshold = std::time::Duration::from_millis(100);

            // Split pipelines for input/output tasks.
            let stats = pipelines.stats.clone();
            let mut input_pipeline = pipelines.input;
            let mut output_pipeline = pipelines.output;

            // Sender used by the capture/input task.
            let mut sender = RealtimeSender {
                encoder: self.encoder.take().expect("encoder missing"),
                writer_tx: self.transport.clone_sender(),
                protocol: self.protocol.clone(),
            };

            let cancel_input = cancel.clone();
            let stats_input = stats.clone();
            let input_task: JoinHandle<Result<Box<dyn Pipeline>>> = tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel_input.cancelled() => break,
                        audio = capture_stream.next() => {
                            match audio {
                                Some(audio) => {
                                    let Some((audio, _)) = input_pipeline.process_stream(audio)? else {
                                        continue
                                    };
                                    // Send audio directly from the input task.
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
                                if !cancel.is_cancelled() {
                                    eprintln!("Transport error: {}", e);
                                }
                                break;
                            }
                        };
                        match event {
                            SessionEvent::SessionStarted(_) => {
                                // Apply session config via `session.update`:
                                // instructions, input transcription, turn
                                // detection, and output modalities.
                                self.send(SessionUpdate(
                                    SessionUpdateEvent {
                                        event_type: "session.update",
                                        session: SessionConfig {
                                            session_type: Some("realtime"),
                                            model: self.config.model.clone(),
                                            instructions: self.config.instructions.clone(),
                                            input_audio_transcription: self.config.transcription_model.as_ref().map(|m| {
                                                InputAudioTranscription { model: m.clone() }
                                            }),
                                            turn_detection: Some(self.config.turn_mode.clone()),
                                            audio: AudioConfig {
                                                input: Some(
                                                    AudioInputConfig {
                                                        format: Some(AudioFormat::pcm_24khz()),
                                                    }
                                                ),
                                                output: AudioOutputConfig {
                                                    format: Some(AudioFormat::pcm_24khz()),
                                                    voice: self.config.voice.clone(),
                                                },
                                            },
                                            output_modalities: Some(vec![
                                                OutputModality::Text,
                                                OutputModality::Audio,
                                            ]),
                                        },
                                    }
                                )).await?;
                            }
                            SessionEvent::SessionConfigured(_) => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::SessionConfigured("Realtime session configured".to_string()));
                                }
                            }
                            SessionEvent::Audio(audio) => {
                                let Some((audio, pipeline_duration)) = output_pipeline.process_stream(audio)? else {
                                    continue
                                };

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
                                                res = playback_sink.send(a) => {
                                                    res.map_err(|e| CoreError::Internal(format!("playback sink error: {}", e)))?;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    tokio::select! {
                                        _ = cancel.cancelled() => break 'main_loop,
                                        res = playback_sink.send(audio) => {
                                            res.map_err(|e| CoreError::Internal(format!("playback sink error: {}", e)))?;
                                        }
                                    }
                                }
                            }
                            SessionEvent::Text(delta) => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::Text(delta));
                                }
                            }
                            SessionEvent::InputText(delta) => {
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::InputText(delta));
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
                                is_playing = false;
                                jitter_buffer.clear();
                                stats.set_output_active(true);
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::ResponseStarted);
                                }
                            }
                            SessionEvent::ResponseFinished => {
                                is_playing = false;
                                stats.set_output_active(false);
                                if let Some(tx) = &event_tx {
                                    let _ = tx.send(SessionEvent::ResponseFinished);
                                }
                            }
                        }
                    }
                }
            }

            self.close().await?;

            // Reassemble pipelines for the caller.
            let input_pipeline = input_task.await
                .map_err(|e| CoreError::Internal(format!("input task panicked: {}", e)))??;

            Ok(Pipelines {
                input: input_pipeline,
                output: output_pipeline,
                stats,
                pool: pipelines.pool,
            })
        })
    }
}

#[async_trait]
impl Session for RealtimeSession {
    async fn send_audio(&mut self, audio: Audio) -> Result<()> {
        if self.closed {
            return Err(CoreError::Internal("session closed".to_string()));
        }

        let encoder = self.encoder.as_mut().ok_or_else(|| CoreError::Internal("encoder taken".to_string()))?;

        let pcm = audio.to_pcm()?;
        let encoded = encoder.encode(&pcm)?;

        self.send(InputAudioBufferAppend::new(encoded.bytes().clone()))
            .await?;

        Ok(())
    }

    async fn next_event(&mut self) -> Result<SessionEvent> {
        if self.closed {
            return Err(CoreError::Internal("session closed".to_string()));
        }
        loop {
            let data = self.transport.recv().await?;

            let event = self.protocol.decode(data.clone())?;
            if let Some(event) = self.map_event(event)? {
                return Ok(event);
            } else {
                eprintln!("ERROR MAP EVENT: {:#?}", data);
            }
        }
    }

    async fn close(&mut self) -> Result<()> {
        if self.closed {
            return Err(CoreError::Internal("session closed".to_string()));
        }

        self.closed = true;

        let _ = self.transport.close().await;
        Ok(())
    }
}

impl RealtimeSession {
    /// Open a Realtime WebSocket and prepare PCM encode/decode for the config format.
    ///
    /// # Errors
    ///
    /// Returns protocol errors if the handshake request cannot be built, transport
    /// errors if the WebSocket connect fails, or codec errors if encoder/decoder
    /// construction fails for [`OpenAIRealtimeConfig::audio_format`].
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
        let encoded = EncodedAudio::new(self.decoder.format().clone(), delta.into_bytes().into())?;
        let pcm = self.decoder.decode(&encoded)?;

        let audio = Audio::from_pcm(&pcm)?;

        Ok(SessionEvent::Audio(audio))
    }

    fn map_event(&mut self, event: ProtocolEvent) -> Result<Option<SessionEvent>> {
        match event {
            ProtocolEvent::SessionCreated { .. } => Ok(Some(SessionEvent::SessionStarted(
                "Realtime session created".to_string(),
            ))),
            ProtocolEvent::SessionUpdated { .. } => Ok(Some(SessionEvent::SessionConfigured(
                "Realtime session configured".to_string(),
            ))),
            ProtocolEvent::ResponseOutputAudioDelta { delta } => Ok(Some(self.map_audio(delta)?)),
            ProtocolEvent::ResponseOutputAudioDone => Ok(None),
            ProtocolEvent::ResponseTextDelta { delta } => Ok(Some(SessionEvent::Text(delta))),
            ProtocolEvent::ResponseTextDone { .. } => Ok(None),
            ProtocolEvent::ConversationItemCreated { item } => {
                // Extract transcribed input text from the first content item
                // that has a non-empty transcript (type "input_audio").
                for content in &item.content {
                    if let Some(transcript) = &content.transcript {
                        if !transcript.is_empty() {
                            return Ok(Some(SessionEvent::InputText(transcript.clone())));
                        }
                    }
                }
                Ok(None)
            }
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