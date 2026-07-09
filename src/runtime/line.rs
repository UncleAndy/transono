use anyhow::anyhow;
use tokio_util::sync::CancellationToken;

use crate::audio::{AudioPipeline, Processor, AudioInput, AudioOutput, Pipelines};
use crate::core::provider::{Provider, ProviderSession};
use crate::runtime::LineState;
use crate::core::error::{CoreError, Result};

pub struct TranslationLine<P>
where
    P: Provider,
{
    provider: P,

    cancel: CancellationToken,

    audio_input: Option<Box<dyn AudioInput>>,
    audio_output: Option<Box<dyn AudioOutput>>,

    pipelines: Option<Pipelines>,

    session_task: Option<tokio::task::JoinHandle<Result<Pipelines>>>,

    state: LineState,
}

impl<P: Provider> TranslationLine<P> {
    pub async fn new(
        provider: P,
        audio_input: Box<dyn AudioInput>,
        audio_output: Box<dyn AudioOutput>,
    ) -> Result<Self> {
        Ok(Self {
            provider,

            cancel: CancellationToken::new(),

            audio_input: Some(audio_input),
            audio_output: Some(audio_output),

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

        if let Some(playback) = self.audio_output.take() {
            playback.stop()?;
        }

        if let Some(capture) = self.audio_input.take() {
            capture.stop()?;
        }

        if let Some(task) = self.session_task.take() {
            let pipelines = task.await
                .map_err(|_| {
                    CoreError::Other(anyhow!("session task panicked"))
                })?
                .map_err(|_| {
                    CoreError::Other(anyhow!("session task panicked"))
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

        let Some(mut playback) = self.audio_output.take() else {
            return Err(CoreError::Other(anyhow!("audio output not found")));
        };
        let output_tx = playback.take_sender()?;
        playback.start()?;
        self.audio_output = Some(playback);

        let Some(mut input) = self.audio_input.take() else {
            return Err(CoreError::Other(anyhow!("audio input not found")));
        };
        let input_rx = input.take_receiver()?;
        input.start()?;
        self.audio_input = Some(input);

        let pipelines = self
            .pipelines
            .take()
            .expect("pipelines missing");

        let session = self.provider.create_session().await?;

        self.session_task = Some(
            session.spawn(
                input_rx,
                output_tx,
                pipelines,
                self.cancel.clone(),
            )
        );

        self.state = LineState::Running;

        Ok(())
    }
}
