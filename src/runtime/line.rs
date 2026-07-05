use tokio_util::sync::CancellationToken;

use crate::audio::{AudioCapture, AudioPipeline, AudioPlayback, AudioProcessor};
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

    input_pipeline: AudioPipeline,
    output_pipeline: AudioPipeline,

    capture_task: Option<std::thread::JoinHandle<()>>,
    session_task: Option<tokio::task::JoinHandle<Result<()>>>,

    state: LineState,
}

impl TranslationLine<P> {
    pub async fn new(
        provider: P,
        capture: AudioCapture,
        playback: AudioPlayback,
    ) -> Result<Self> {
        Ok(Self {
            provider,

            cancel: CancellationToken::new(),

            capture,
            playback,

            input_pipeline: AudioPipeline::new(),
            output_pipeline: AudioPipeline::new(),

            capture_task: None,
            session_task: None,

            state: LineState::Created,
        })
    }

    pub fn add_input_processor<T>(
        &mut self,
        processor: T,
    ) -> Result<()>
    where
        T: AudioProcessor + 'static,
    {
        if self.state == LineState::Running {
            return Err(CoreError::Other(anyhow::Error::msg("TranslationLine is running")));
        }

        self.input_pipeline.add(processor);

        Ok(())
    }

    pub fn add_output_processor<T>(
        &mut self,
        processor: T,
    ) -> Result<()>
    where
        T: AudioProcessor + 'static,
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

        if let Some(task) = self.capture_task.take() {
            task.join().unwrap();
        }

        if let Some(task) = self.session_task.take() {
            task.await??;
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

        self.capture_task = Some(
            spawn_capture(
                self.capture.clone(),
                self.input_pipeline.clone(),
                audio_tx,
            )?
        );

        self.session_task = Some(
            spawn_session(
                self.provider.create_session().await?,
                self.playback.clone(),
                self.output_pipeline.clone(),
                audio_rx,
                self.cancel.clone(),
            )
        );

        self.state = LineState::Running;

        Ok(())
    }
}

fn spawn_capture(
    mut capture: AudioCapture,
    mut pipeline: AudioPipeline,
    audio_tx: tokio::sync::mpsc::Sender<Audio>,
) -> Result<std::thread::JoinHandle<Result<()>>> {

    Ok(std::thread::spawn(move || -> Result<()> {

        capture.start(|audio| {

            if let Ok(audio) = pipeline.process(audio) {
                let _ = audio_tx.blocking_send(audio);
            }

        })?;

        Ok(())
    }))
}

async fn spawn_session<S>(
    mut session: S,
    mut playback: AudioPlayback,
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

                            playback.play(audio)?;

                        }

                        SessionEvent::RequestStarted => {}

                        SessionEvent::RequestFinished => {}

                        SessionEvent::ResponseStarted => {}

                        SessionEvent::ResponseFinished => {}
                    }
                }
            }
        }

        Ok(())
    })
}
