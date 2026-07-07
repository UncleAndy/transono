use anyhow::anyhow;
use cpal::Device;
use tokio_util::sync::CancellationToken;

use crate::audio::{Audio, AudioCapture, AudioPipeline, AudioPlayback, AudioProcessor, Processor};
use crate::core::provider::Provider;
use crate::runtime::LineState;
use crate::core::error::{CoreError, Result};
use crate::core::session::Session;
use crate::core::session_event::SessionEvent;

pub struct TranslationLine<P>
where
    P: Provider,
{
    provider: P,

    cancel: CancellationToken,

    capture: AudioCapture,
    playback: AudioPlayback,
    playback_tx: tokio::sync::mpsc::Sender<Audio>,

    input_pipeline: AudioPipeline,
    output_pipeline: Option<AudioPipeline>,

    session_task: Option<tokio::task::JoinHandle<Result<()>>>,

    state: LineState,
}

impl<P: Provider> TranslationLine<P> {
    pub async fn new(
        provider: P,
        capture: AudioCapture,
        playback_device: Device,
    ) -> Result<Self> {
        let (playback_tx, playback_rx) =
            tokio::sync::mpsc::channel(32);

        let playback =
            AudioPlayback::new(
                playback_device,
                playback_rx,
            )?;

        Ok(Self {
            provider,

            cancel: CancellationToken::new(),

            capture,
            playback,
            playback_tx,

            input_pipeline: AudioPipeline::new(),
            output_pipeline: Some(AudioPipeline::new()),

            session_task: None,

            state: LineState::Created,
        })
    }

    pub fn add_input_processor(
        &mut self,
        processor: Processor,
    ) -> Result<()>
    {
        if self.state == LineState::Running {
            return Err(CoreError::Other(anyhow::Error::msg("TranslationLine is running")));
        }

        self.input_pipeline.add(processor);

        Ok(())
    }

    pub fn add_output_processor(
        &mut self,
        processor: Processor,
    ) -> Result<()>
    {
        if self.state == LineState::Running {
            return Err(CoreError::Other(anyhow::Error::msg("TranslationLine is running")));
        }

        self.output_pipeline.add(processor);

        Ok(())
    }

    pub fn state(
        &self,
    ) -> LineState {

        self.state
    }

    pub async fn stop(&mut self) -> Result<()> {

        if self.state != LineState::Running {
            return Ok(());
        }

        self.capture.stop()?;

        self.cancel.cancel();

        if let Some(task) = self.session_task.take() {
            task.await
                .map_err(|_| {
                    CoreError::Other(anyhow!("capture thread panicked"))
                })?
                .map_err(|_| {
                    CoreError::Other(anyhow!("capture thread panicked"))
                })?
        }

        self.state = LineState::Stopped;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Ok(());
        }

        self.cancel = CancellationToken::new();

        let (audio_tx, audio_rx) = tokio::sync::mpsc::channel(32);

        self.capture.start()?;
        self.playback.start()?;

        let output_pipeline = self
            .output_pipeline
            .take()
            .expect("output pipeline missing");

        self.session_task = Some(
            spawn_session(
                self.provider.create_session().await?,
                self.playback_tx.clone(),
                output_pipeline,
                audio_rx,
                self.cancel.clone(),
            )
        );

        self.state = LineState::Running;

        Ok(())
    }
}

fn spawn_session<S>(
    mut session: S,
    playback_tx: tokio::sync::mpsc::Sender<Audio>,
    mut pipeline: AudioPipeline,
    mut audio_rx: tokio::sync::mpsc::Receiver<Audio>,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>>
where
    S: Session + 'static,
{
    tokio::spawn(async move {

        loop {

            tokio::select! {

                _ = cancel.cancelled() => {
                    break;
                }

                Some(audio) = audio_rx.recv() => {

                    session.send_audio(audio).await?;

                }

                event = session.next_event() => {

                    match event? {

                        SessionEvent::Audio(audio) => {

                            let audio =
                                pipeline.process(audio)?;

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

        session.close().await?;

        Ok(())
    })
}
