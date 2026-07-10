use anyhow::anyhow;
use tokio_util::sync::CancellationToken;
use std::sync::Arc;

use crate::audio::{Processor, AudioInput, AudioOutput, Pipelines};
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
    latency_stats: Arc<crate::audio::LatencyStats>,

    session_task: Option<tokio::task::JoinHandle<Result<Pipelines>>>,

    state: LineState,
}

impl<P: Provider> TranslationLine<P> {
    pub async fn new(
        provider: P,
        audio_input: Box<dyn AudioInput>,
        audio_output: Box<dyn AudioOutput>,
    ) -> Result<Self> {
        let pipelines = Pipelines::new();
        let latency_stats = pipelines.stats.clone();

        Ok(Self {
            provider,

            cancel: CancellationToken::new(),

            audio_input: Some(audio_input),
            audio_output: Some(audio_output),

            pipelines: Some(pipelines),
            latency_stats,

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

    pub fn latency(&self) -> crate::audio::LatencySnapshot {
        self.latency_stats.snapshot()
    }

    pub async fn stop(&mut self) -> Result<()> {
        if self.state != LineState::Running {
            return Ok(());
        }

        self.cancel.cancel();
 
        if let Some(task) = self.session_task.take() {
            let res = task.await
                .map_err(|e| {
                    if e.is_panic() {
                        CoreError::Other(anyhow!("session task panicked"))
                    } else {
                        CoreError::Other(anyhow!("session task error: {}", e))
                    }
                })?;
 
            match res {
                Ok(pipelines) => {
                    self.pipelines = Some(pipelines);
                }
                Err(e) => {
                    // Даже если сессия завершилась с ошибкой, мы должны остановить аудио
                    let _ = self.stop_audio();
                    return Err(e);
                }
            }
        }

        self.stop_audio()?;
 
        self.state = LineState::Stopped;
 
        Ok(())
    }

    fn stop_audio(&mut self) -> Result<()> {
        if let Some(playback) = self.audio_output.take() {
            playback.stop()?;
        }

        if let Some(capture) = self.audio_input.take() {
            capture.stop()?;
        }

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Ok(());
        }

        self.cancel = CancellationToken::new();

        let session = self.provider.create_session().await?;

        let Some(mut playback) = self.audio_output.take() else {
            return Err(CoreError::Other(anyhow!("audio output not found")));
        };
        let output_tx = playback.clone_sender()?;
        playback.start()?;
        self.audio_output = Some(playback);

        let Some(mut input) = self.audio_input.take() else {
            return Err(CoreError::Other(anyhow!("audio input not found")));
        };
        let mut input_rx = input.take_receiver()?;

        // Clear any stale audio data that might have been captured during initialization
        while input_rx.try_recv().is_ok() {}

        input.start()?;
        self.audio_input = Some(input);

        let pipelines = self
            .pipelines
            .take()
            .expect("pipelines missing");

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
