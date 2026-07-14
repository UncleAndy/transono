use tokio_util::sync::CancellationToken;
use std::sync::Arc;

use tokio::sync::mpsc;
use crate::audio::{Processor, AudioInput, AudioOutput, Pipelines, LatencyStats, Pipeline, AudioFormat, AudioPipeline};
use crate::core::provider::{Provider, ProviderSession};
use crate::line::LineState;
use crate::core::error::{CoreError, Result};
use crate::core::session_event::SessionEvent;

pub struct TranslationLine<P>
where
    P: Provider,
{
    provider: P,

    cancel: CancellationToken,

    audio_input: Option<Box<dyn AudioInput>>,
    audio_output: Option<Box<dyn AudioOutput>>,

    pipelines: Option<Pipelines>,
    pub latency_stats: Arc<LatencyStats>,

    session_task: Option<tokio::task::JoinHandle<Result<Pipelines>>>,

    state: LineState,

    event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
}

impl<P: Provider> TranslationLine<P> {
    pub async fn new(
        provider: P,
        audio_input: Box<dyn AudioInput>,
        audio_output: Box<dyn AudioOutput>,
        stats: Arc<LatencyStats>,
    ) -> Result<Self> {
        let pipelines = Pipelines::with_stats(stats.clone());
        let latency_stats = stats;

        let mut line = Self {
            provider,

            cancel: CancellationToken::new(),

            audio_input: Some(audio_input),
            audio_output: Some(audio_output),

            pipelines: Some(pipelines),
            latency_stats,

            session_task: None,

            state: LineState::Created,

            event_tx: None,
        };

        line.auto_configure()?;

        Ok(line)
    }

    pub fn set_event_sender(&mut self, tx: mpsc::UnboundedSender<SessionEvent>) {
        self.event_tx = Some(tx);
    }

    pub fn add_input_processor(
        &mut self,
        processor: Processor,
    ) -> Result<()>
    {
        if self.state == LineState::Running {
            return Err(CoreError::Internal("TranslationLine is running".to_string()));
        }

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.input.add(processor);
        }

        Ok(())
    }

    pub fn with_input_proc(
        &mut self,
        processor: Processor,
    ) -> &mut Self {
        if let Err(e) = self.add_input_processor(processor) {
            eprintln!("Can not add input processor to line: {}", e);
        }
 
        self
    }

    pub fn with_input_pipeline(
        &mut self,
        pipeline: Box<dyn Pipeline>
    ) -> &mut Self {
        if self.state == LineState::Running {
            eprintln!("TranslationLine is running");
            return self;
        }
 
        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.input = pipeline;
        }
 
        self
    }

    pub fn add_output_processor(
        &mut self,
        processor: Processor,
    ) -> Result<()>
    {
        if self.state == LineState::Running {
            return Err(CoreError::Internal("TranslationLine is running".to_string()));
        }

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.output.add(processor);
        }

        Ok(())
    }

    pub fn clear_input_processors(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Err(CoreError::Internal("TranslationLine is running".to_string()));
        }

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.input.clear();
        }

        Ok(())
    }

    pub fn clear_output_processors(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Err(CoreError::Internal("TranslationLine is running".to_string()));
        }

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.output.clear();
        }

        Ok(())
    }

    pub fn with_output_proc(
        &mut self,
        processor: Processor,
    ) -> &mut Self {
        if let Err(e) = self.add_output_processor(processor) {
            eprintln!("Can not add output processor to line: {}", e);
        }

        self
    }

    pub fn with_output_pipeline(
        &mut self,
        pipeline: Box<dyn Pipeline>
    ) -> &mut Self {
        if self.state == LineState::Running {
            eprintln!("TranslationLine is running");
            return self;
        }
 
        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.output = pipeline;
        }
 
        self
    }

    pub fn state(
        &self,
    ) -> LineState {
        self.state
    }

    pub fn latency(&self) -> crate::audio::LatencySnapshot {
        self.latency_stats.snapshot()
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn audio_input(&self) -> Option<&dyn AudioInput> {
        self.audio_input.as_deref()
    }

    pub fn audio_output(&self) -> Option<&dyn AudioOutput> {
        self.audio_output.as_deref()
    }

    pub fn auto_configure(&mut self) -> Result<()> {
        let input_format = self.audio_input.as_ref().map(|i| i.format()).ok_or_else(|| CoreError::Internal("audio input missing".to_string()))?;
        let output_format = self.audio_output.as_ref().map(|o| o.format()).ok_or_else(|| CoreError::Internal("audio output missing".to_string()))?;
        let provider_format = AudioFormat::from(self.provider.audio_format());

        // Configure Input Pipeline: HW -> Provider
        let input_pipeline = AudioPipeline::new_input_pipeline(self.latency_stats.clone(), input_format, provider_format)?;
        self.with_input_pipeline(input_pipeline);

        // Configure Output Pipeline: Provider -> HW
        let output_pipeline = AudioPipeline::new_output_pipeline(self.latency_stats.clone(), provider_format, output_format)?;
        self.with_output_pipeline(output_pipeline);

        Ok(())
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
                        CoreError::Internal("session task panicked".to_string())
                    } else {
                        CoreError::Internal(format!("session task error: {}", e))
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
            return Err(CoreError::Internal("audio output not found".to_string()));
        };
        let playback_sink = playback.sink()?;
        playback.start()?;
        self.audio_output = Some(playback);

        let Some(mut input) = self.audio_input.take() else {
            return Err(CoreError::Internal("audio input not found".to_string()));
        };
        let input_stream = input.stream()?;

        input.start()?;
        self.audio_input = Some(input);

        let pipelines = self
            .pipelines
            .take()
            .expect("pipelines missing");

        self.session_task = Some(
            session.spawn(
                input_stream,
                playback_sink,
                pipelines,
                self.cancel.clone(),
                self.event_tx.clone(),
            )
        );

        self.state = LineState::Running;
 
        Ok(())
    }
}
 
impl<P: Provider> Drop for TranslationLine<P> {
    fn drop(&mut self) {
        self.cancel.cancel();
        let _ = self.stop_audio();
    }
}
