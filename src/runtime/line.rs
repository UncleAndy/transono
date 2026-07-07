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

    capture_device: Device,
    playback_device: Device,

    input_pipeline: Option<AudioPipeline>,
    output_pipeline: Option<AudioPipeline>,

    session_task: Option<tokio::task::JoinHandle<Result<()>>>,

    state: LineState,
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

            capture_device,
            playback_device,

            input_pipeline: Some(AudioPipeline::new()),
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

        if let Some(pipeline) = self.input_pipeline.as_mut() {
            pipeline.add(processor);
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

        if let Some(pipeline) = self.output_pipeline.as_mut() {
            pipeline.add(processor);
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

        let (playback, playback_tx) =
            AudioPlayback::new(self.playback_device.clone())?;

        let (capture, capture_rx) =
            AudioCapture::new(self.capture_device.clone())?;

        self.cancel = CancellationToken::new();

        capture.start()?;
        playback.start()?;

        let input_pipeline = self
            .input_pipeline
            .take()
            .expect("output pipeline missing");

        let output_pipeline = self
            .output_pipeline
            .take()
            .expect("output pipeline missing");

        self.session_task = Some(
            spawn_session(
                self.provider.create_session().await?,
                capture_rx,
                playback_tx,
                input_pipeline,
                output_pipeline,
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
    mut input_pipeline: AudioPipeline,
    mut output_pipeline: AudioPipeline,
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

                Some(audio) = capture_rx.recv() => {
                    let audio =
                                input_pipeline.process(audio)?;

                    session.send_audio(audio).await?;
                }

                event = session.next_event() => {
                    match event? {
                        SessionEvent::Audio(audio) => {
                            let audio =
                                output_pipeline.process(audio)?;

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
