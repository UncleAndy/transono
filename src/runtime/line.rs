use anyhow::anyhow;
use cpal::Device;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::audio::{
    Audio,
    AudioCapture,
    AudioPipeline,
    AudioPlayback,
    Processor,
};
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

    capture: Option<AudioCapture>,
    capture_device: Device,

    playback: Option<AudioPlayback>,
    playback_device: Device,

    pipelines: Option<Pipelines>,

    session_task: Option<tokio::task::JoinHandle<Result<Pipelines>>>,

    state: LineState,
}

struct Pipelines {
    input: AudioPipeline,
    output: AudioPipeline,
}

impl<P: Provider> TranslationLine<P> {
    pub async fn new(
        provider: P,
        capture_device: Device,
        playback_device: Device,
    ) -> Result<Self> {
        Ok(Self {
            provider,

            cancel: CancellationToken::new(),

            capture: None,
            capture_device,

            playback: None,
            playback_device,

            pipelines: Some(Pipelines {
                input: AudioPipeline::new(),
                output: AudioPipeline::new(),
            }),

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

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.input.add(processor);
        }

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

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.output.add(processor);
        }

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

        self.cancel.cancel();

        if let Some(playback) = self.playback.take() {
            playback.stop()?;
        }

        if let Some(capture) = self.capture.take() {
            capture.stop()?;
        }

        if let Some(task) = self.session_task.take() {
            let pipelines = task.await
                .map_err(|_| {
                    CoreError::Other(anyhow!("capture thread panicked"))
                })?
                .map_err(|_| {
                    CoreError::Other(anyhow!("capture thread panicked"))
                })?;

            self.pipelines = Some(pipelines);
        }

        self.state = LineState::Stopped;

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Ok(());
        }

        self.cancel = CancellationToken::new();

        let (playback, playback_tx) =
            AudioPlayback::new(self.playback_device.clone())?;
        playback.start()?;
        self.playback = Some(playback);

        let (capture, capture_rx) =
            AudioCapture::new(self.capture_device.clone())?;
        capture.start()?;
        self.capture = Some(capture);

        let pipelines = self
            .pipelines
            .take()
            .expect("pipelines missing");

        self.session_task = Some(
            spawn_session(
                self.provider.create_session().await?,
                capture_rx,
                playback_tx,
                pipelines,
                self.cancel.clone(),
            )
        );

        self.state = LineState::Running;

        Ok(())
    }
}

fn spawn_session<S>(
    mut session: S,
    mut capture_rx: mpsc::Receiver<Audio>,
    playback_tx: mpsc::Sender<Audio>,
    mut pipelines: Pipelines,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<Result<Pipelines>>
where
    S: Session + 'static,
{
    tokio::spawn(async move {

        loop {

            tokio::select! {

                _ = cancel.cancelled() => {
                    break;
                }

                Some(audio) = capture_rx.recv() => {
                    let audio = pipelines.input.process(audio)?;

                    session.send_audio(audio).await?;
                }

                event = session.next_event() => {
                    match event? {
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

        session.close().await?;

        Ok(pipelines)
    })
}
