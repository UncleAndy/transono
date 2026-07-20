//! [`TranslationLine`]: one capture→provider→playback speech-translation stream.

use tokio_util::sync::CancellationToken;
use std::sync::Arc;

use tokio::sync::mpsc;
use crate::audio::{Processor, AudioInput, AudioOutput, Pipelines, LatencyStats, Pipeline, AudioFormat, AudioPipeline};
use crate::core::provider::{Provider, ProviderSession};
use crate::line::LineState;
use crate::core::error::{CoreError, Result};
use crate::core::session_event::SessionEvent;

/// Single capture→provider→playback translation stream.
///
/// Parameterized by a [`Provider`]. Owns audio devices, DSP
/// [`Pipelines`], and an optional provider session task. Prefer configuring
/// processors before [`Self::run`].
///
/// # Examples
///
/// ```no_run
/// // Requires real audio devices and provider credentials.
/// # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
/// # Ok(())
/// # }
/// ```
pub struct TranslationLine<P>
where
    P: Provider,
{
    provider: P,

    cancel: CancellationToken,

    audio_input: Option<Box<dyn AudioInput>>,
    audio_output: Option<Box<dyn AudioOutput>>,

    pipelines: Option<Pipelines>,
    /// Shared latency counters updated by the DSP pipelines.
    pub latency_stats: Arc<LatencyStats>,

    session_task: Option<tokio::task::JoinHandle<Result<Pipelines>>>,

    state: LineState,

    event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
}

impl<P: Provider> TranslationLine<P> {
    /// Build a line with default input/output pipelines for the given devices.
    ///
    /// Creates [`Pipelines`] backed by `stats`, then calls [`Self::auto_configure`]
    /// so capture and playback formats match the provider.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] if pipeline auto-configuration fails (missing
    /// audio devices or unsupported format conversion).
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

    /// Attach an unbounded channel for [`SessionEvent`]s from the provider session.
    pub fn set_event_sender(&mut self, tx: mpsc::UnboundedSender<SessionEvent>) {
        self.event_tx = Some(tx);
    }

    /// Append a DSP stage on the capture (input) path.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if the line is already [`LineState::Running`].
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

    /// Append an input processor, printing to stderr on failure instead of returning an error.
    ///
    /// When the line is [`LineState::Running`], the failure from
    /// [`Self::add_input_processor`] is logged via `eprintln!` and ignored.
    pub fn with_input_proc(
        &mut self,
        processor: Processor,
    ) -> &mut Self {
        if let Err(e) = self.add_input_processor(processor) {
            eprintln!("Can not add input processor to line: {}", e);
        }
 
        self
    }

    /// Replace the entire capture pipeline.
    ///
    /// No-ops (after `eprintln!`) when the line is [`LineState::Running`].
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

    /// Append a DSP stage on the playback (output) path.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if the line is already [`LineState::Running`].
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

    /// Remove all processors from the capture pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if the line is already [`LineState::Running`].
    pub fn clear_input_processors(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Err(CoreError::Internal("TranslationLine is running".to_string()));
        }

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.input.clear();
        }

        Ok(())
    }

    /// Remove all processors from the playback pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if the line is already [`LineState::Running`].
    pub fn clear_output_processors(&mut self) -> Result<()> {
        if self.state == LineState::Running {
            return Err(CoreError::Internal("TranslationLine is running".to_string()));
        }

        if let Some(pipelines) = self.pipelines.as_mut() {
            pipelines.output.clear();
        }

        Ok(())
    }

    /// Append an output processor, printing to stderr on failure instead of returning an error.
    ///
    /// When the line is [`LineState::Running`], the failure from
    /// [`Self::add_output_processor`] is logged via `eprintln!` and ignored.
    pub fn with_output_proc(
        &mut self,
        processor: Processor,
    ) -> &mut Self {
        if let Err(e) = self.add_output_processor(processor) {
            eprintln!("Can not add output processor to line: {}", e);
        }

        self
    }

    /// Replace the entire playback pipeline.
    ///
    /// No-ops (after `eprintln!`) when the line is [`LineState::Running`].
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

    /// Current lifecycle state.
    pub fn state(
        &self,
    ) -> LineState {
        self.state
    }

    /// Snapshot of latency counters from [`Self::latency_stats`].
    pub fn latency(&self) -> crate::audio::LatencySnapshot {
        self.latency_stats.snapshot()
    }

    /// Borrow the configured provider.
    pub fn provider(&self) -> &P {
        &self.provider
    }

    /// Borrow the capture device, if still attached.
    pub fn audio_input(&self) -> Option<&dyn AudioInput> {
        self.audio_input.as_deref()
    }

    /// Borrow the playback device, if still attached.
    pub fn audio_output(&self) -> Option<&dyn AudioOutput> {
        self.audio_output.as_deref()
    }

    /// Rebuild default DSP pipelines for hardware ↔ provider format conversion.
    ///
    /// Capture path: device format → provider format. Playback path: provider
    /// format → device format. Called automatically from [`Self::new`].
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if audio input/output is missing, or
    /// pipeline construction fails for the negotiated formats.
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

    /// Cancel the provider session, reclaim pipelines, and stop audio devices.
    ///
    /// No-ops if the line is not [`LineState::Running`]. On success the state
    /// becomes [`LineState::Stopped`].
    ///
    /// # Errors
    ///
    /// Propagates session-task join/panic failures, session [`CoreError`]s, or
    /// audio stop failures. If the session ended with an error, audio is still
    /// stopped before returning that error.
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
                    // Even if the session ended with an error, we must stop audio.
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
        if let Some(mut playback) = self.audio_output.take() {
            playback.stop()?;
        }

        if let Some(capture) = self.audio_input.take() {
            capture.stop()?;
        }

        Ok(())
    }

    /// Start capture, playback, and the provider session.
    ///
    /// Creates a provider session, starts audio I/O, transfers ownership of
    /// [`Pipelines`] into the session task, and sets state to
    /// [`LineState::Running`]. No-ops if already running.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] if session creation fails, audio devices are
    /// missing, or starting capture/playback fails.
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
