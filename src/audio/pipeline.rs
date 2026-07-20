use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use symphonia::core::audio::AudioSpec;

use crate::audio::processors::channel_converter::ChannelConverter;
use crate::audio::processors::resampler::Resampler;
use crate::audio::{
    Audio, AudioFormat, AudioProcessor, DspProcessor, PcmAudio,
    Pipeline, Processor, SharedPcmPool, PcmPool,
};
use crate::core::error::{CoreError, Result};

/// Metric for tracking latency values.
#[derive(Debug)]
pub struct LatencyMetric {
    /// Minimum latency in microseconds.
    pub min_us: AtomicU64,
    /// Maximum latency in microseconds.
    pub max_us: AtomicU64,
    /// Cumulative latency sum in microseconds.
    pub sum_us: AtomicU64,
    /// Number of measurements taken.
    pub count: AtomicU64,
    /// Last recorded latency in microseconds.
    pub last_us: AtomicU64,
}

impl Default for LatencyMetric {
    fn default() -> Self {
        Self {
            min_us: AtomicU64::new(u64::MAX),
            max_us: AtomicU64::new(0),
            sum_us: AtomicU64::new(0),
            count: AtomicU64::new(0),
            last_us: AtomicU64::new(0),
        }
    }
}

impl LatencyMetric {
    /// Updates the metric with a new latency value in microseconds.
    pub fn update(&self, value_us: u64) {
        self.min_us.fetch_min(value_us, Ordering::Relaxed);
        self.max_us.fetch_max(value_us, Ordering::Relaxed);
        self.sum_us.fetch_add(value_us, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.last_us.store(value_us, Ordering::Relaxed);
    }

    /// Takes a snapshot of the current metric values.
    pub fn snapshot(&self) -> MetricSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let min = self.min_us.load(Ordering::Relaxed);
        MetricSnapshot {
            min_ms: if min == u64::MAX {
                0.0
            } else {
                min as f64 / 1000.0
            },
            max_ms: self.max_us.load(Ordering::Relaxed) as f64 / 1000.0,
            avg_ms: if count > 0 {
                (self.sum_us.load(Ordering::Relaxed) as f64 / count as f64) / 1000.0
            } else {
                0.0
            },
            last_ms: self.last_us.load(Ordering::Relaxed) as f64 / 1000.0,
        }
    }
}

/// Collection of latency metrics for different parts of the system.
#[derive(Debug, Default)]
pub struct LatencyStats {
    /// Latency of the input processing pipeline.
    pub input_pipeline: LatencyMetric,
    /// Total input latency including capture.
    pub input_total: LatencyMetric,
    /// Latency of the output processing pipeline.
    pub output_pipeline: LatencyMetric,
    /// Total output latency including playback.
    pub output_total: LatencyMetric,
    /// Number of frames dropped at input.
    pub dropped_input: AtomicU64,
    /// Number of packets dropped during network transmission.
    pub dropped_network: AtomicU64,
    /// Number of frames dropped at output.
    pub dropped_output: AtomicU64,
    /// Whether the output stream is currently active.
    pub output_active: AtomicBool,
}

/// Snapshot of a single latency metric.
#[derive(Debug, Clone, Copy, Default)]
pub struct MetricSnapshot {
    /// Minimum latency in milliseconds.
    pub min_ms: f64,
    /// Maximum latency in milliseconds.
    pub max_ms: f64,
    /// Average latency in milliseconds.
    pub avg_ms: f64,
    /// Last latency in milliseconds.
    pub last_ms: f64,
}

/// Snapshot of all latency statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct LatencySnapshot {
    /// Snapshot of input pipeline metrics.
    pub input_pipeline: MetricSnapshot,
    /// Snapshot of total input metrics.
    pub input_total: MetricSnapshot,
    /// Snapshot of output pipeline metrics.
    pub output_pipeline: MetricSnapshot,
    /// Snapshot of total output metrics.
    pub output_total: MetricSnapshot,
    /// Total frames dropped at input.
    pub dropped_input: u64,
    /// Total packets dropped on the network.
    pub dropped_network: u64,
    /// Total frames dropped at output.
    pub dropped_output: u64,
}

impl LatencyStats {
    /// Takes a snapshot of all statistics.
    pub fn snapshot(&self) -> LatencySnapshot {
        LatencySnapshot {
            input_pipeline: self.input_pipeline.snapshot(),
            input_total: self.input_total.snapshot(),
            output_pipeline: self.output_pipeline.snapshot(),
            output_total: self.output_total.snapshot(),
            dropped_input: self.dropped_input.load(Ordering::Relaxed),
            dropped_network: self.dropped_network.load(Ordering::Relaxed),
            dropped_output: self.dropped_output.load(Ordering::Relaxed),
        }
    }

    /// Increments the dropped input counter.
    pub fn inc_dropped_input(&self) {
        self.dropped_input.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the dropped network counter.
    pub fn inc_dropped_network(&self) {
        self.dropped_network.fetch_add(1, Ordering::Relaxed);
    }

    /// Increments the dropped output counter.
    pub fn inc_dropped_output(&self) {
        if self.output_active.load(Ordering::Relaxed) {
            self.dropped_output.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Sets whether the output is currently active.
    pub fn set_output_active(&self, active: bool) {
        self.output_active.store(active, Ordering::Relaxed);
    }
}

/// Audio processing pipeline that executes a sequence of processors.
pub struct AudioPipeline {
    processors: Vec<Processor>,
    scratch_pcm: Option<PcmAudio>,
    stats: Arc<LatencyStats>,
    is_input: bool,
    pool: SharedPcmPool,
}

impl AudioPipeline {
    /// Creates a new audio pipeline.
    pub fn new(stats: Arc<LatencyStats>, is_input: bool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats,
            is_input,
            pool: Arc::new(PcmPool::new()),
        }
    }
 
    /// Creates a new audio pipeline with a specific PCM pool.
    pub fn with_pool(stats: Arc<LatencyStats>, is_input: bool, pool: SharedPcmPool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats,
            is_input,
            pool,
        }
    }
 
    /// Creates a standalone audio pipeline without external stats.
    pub fn new_standalone(is_input: bool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats: Arc::new(LatencyStats::default()),
            is_input,
            pool: Arc::new(PcmPool::new()),
        }
    }

    /// Adds a processor to the pipeline.
    pub fn with(&mut self, processor: Processor) -> &mut Self {
        self.processors.push(processor);
        self
    }

    /// Creates a pipeline for converting input audio between formats.
    pub fn new_input_pipeline(stats: Arc<LatencyStats>, from: AudioFormat, to: AudioFormat) -> Result<Box<Self>> {
        let mut pipeline = Self::new(stats, true);

        let mut current_spec = from.spec();

        // Преобразование каналов (делаем первым для оптимизации ресемплинга)
        if from.channels != to.channels {
            pipeline.with(Processor::ChannelConverter(ChannelConverter::new(
                to.spec().channels().clone(),
            )));
            current_spec = AudioSpec::new(from.sample_rate, to.spec().channels().clone());
        }

        // Ресемплирование
        if from.sample_rate != to.sample_rate {
            pipeline.with(Processor::Resampler(Resampler::new(
                current_spec,
                to.sample_rate,
            )?));
        }

        Ok(Box::new(pipeline))
    }

    /// Creates a pipeline for converting output audio between formats.
    pub fn new_output_pipeline(stats: Arc<LatencyStats>, from: AudioFormat, to: AudioFormat) -> Result<Box<Self>> {
        let mut pipeline = Self::new(stats, false);

        // Ресемплирование
        if from.sample_rate != to.sample_rate {
            pipeline.with(Processor::Resampler(Resampler::new(
                from.spec(),
                to.sample_rate,
            )?));
        }

        // Преобразование в целевую конфигурацию каналов
        if from.channels != to.channels {
            pipeline.with(Processor::ChannelConverter(ChannelConverter::new(
                to.spec().channels().clone(),
            )));
        }

        Ok(Box::new(pipeline))
    }

    /// Processes an audio stream and updates latency statistics.
    pub fn process_stream(&mut self, mut audio: Audio) -> Result<Option<(Audio, Duration)>> {
        let start_time = Instant::now();

        if !self.process_audio(&mut audio)? {
            return Ok(None);
        }

        let duration = start_time.elapsed();
        let duration_us = duration.as_micros() as u64;

        if self.is_input {
            self.stats.input_pipeline.update(duration_us);
            self.stats
                .input_total
                .update(audio.capture_timestamp().elapsed().as_micros() as u64);
        } else {
            self.stats.output_pipeline.update(duration_us);
            self.stats
                .output_total
                .update(audio.capture_timestamp().elapsed().as_micros() as u64);
        }

        Ok(Some((audio, duration)))
    }

    /// Returns true if the pipeline has no processors.
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    /// Removes all processors from the pipeline.
    pub fn clear(&mut self) {
        self.processors.clear()
    }

    /// Internal processing method that can be called from traits.
    pub fn process_audio(&mut self, audio: &mut Audio) -> Result<bool> {
        let mut in_scratch = false;
 
        for processor in &mut self.processors {
            if processor.is_audio() {
                if in_scratch {
                    let scratch = self
                        .scratch_pcm
                        .as_ref()
                        .ok_or_else(|| CoreError::Internal("scratch_pcm missing".to_string()))?;
                    *audio = Audio::from_pcm(scratch)?;
                    in_scratch = false;
                }
 
                if !processor.process_audio(audio)? {
                    return Ok(false);
                }
            } else {
                if !in_scratch {
                    if let Some(ref mut scratch) = self.scratch_pcm {
                        audio.to_pcm_into(scratch)?;
                    } else {
                        let mut scratch = self.pool.acquire(audio.buffer().spec().clone(), audio.buffer().frames());
                        audio.to_pcm_into(&mut scratch)?;
                        self.scratch_pcm = Some(scratch);
                    }
                    in_scratch = true;
                }
 
                let scratch = self
                    .scratch_pcm
                    .as_mut()
                    .ok_or_else(|| CoreError::Internal("scratch_pcm missing".to_string()))?;
                if !processor.process_dsp(scratch)? {
                    return Ok(false);
                }
            }
        }
 
        if in_scratch {
            let scratch = self
                .scratch_pcm
                .as_ref()
                .ok_or_else(|| CoreError::Internal("scratch_pcm missing".to_string()))?;
            *audio = Audio::from_pcm(scratch)?;
        }
 
        Ok(true)
    }

    /// Processes DSP-level PCM audio.
    pub fn process_dsp(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        let mut in_audio = false;
        let mut current_audio: Option<Audio> = None;

        for processor in &mut self.processors {
            if processor.is_audio() {
                if !in_audio {
                    current_audio = Some(Audio::from_pcm(pcm)?);
                    in_audio = true;
                }

                if !processor.process_audio(current_audio.as_mut().ok_or_else(|| CoreError::Internal("current_audio missing".to_string()))?)? {
                    return Ok(false);
                }
            } else {
                if in_audio {
                    current_audio.take().ok_or_else(|| CoreError::Internal("current_audio missing".to_string()))?.to_pcm_into(pcm)?;
                    in_audio = false;
                }

                if !processor.process_dsp(pcm)? {
                    return Ok(false);
                }
            }
        }

        if in_audio {
            current_audio.ok_or_else(|| CoreError::Internal("current_audio missing".to_string()))?.to_pcm_into(pcm)?;
        }

        Ok(true)
    }
}

impl AudioProcessor for AudioPipeline {
    fn process(&mut self, input: &mut Audio) -> Result<bool> {
        self.process_audio(input)
    }
}

impl DspProcessor for AudioPipeline {
    fn process(&mut self, input: &mut PcmAudio) -> Result<bool> {
        self.process_dsp(input)
    }
}

impl Pipeline for AudioPipeline {
    fn add(&mut self, processor: Processor) {
        self.with(processor);
    }

    fn clear(&mut self) {
        self.processors.clear();
    }

    fn process_stream(&mut self, audio: Audio) -> Result<Option<(Audio, Duration)>> {
        self.process_stream(audio)
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new_standalone(true)
    }
}

/// Container for input and output pipelines.
pub struct Pipelines {
    /// The input audio processing pipeline.
    pub input: Box<dyn Pipeline>,
    /// The output audio processing pipeline.
    pub output: Box<dyn Pipeline>,
    /// Shared latency statistics for both pipelines.
    pub stats: Arc<LatencyStats>,
    /// Shared PCM buffer pool.
    pub pool: SharedPcmPool,
}
 
impl Pipelines {
    /// Creates a new set of pipelines with default statistics.
    pub fn new() -> Self {
        Self::with_stats(Arc::new(LatencyStats::default()))
    }
 
    /// Creates a new set of pipelines with specified statistics.
    pub fn with_stats(stats: Arc<LatencyStats>) -> Self {
        let pool = Arc::new(PcmPool::new());
        Self {
            input: Box::new(AudioPipeline::with_pool(stats.clone(), true, pool.clone())),
            output: Box::new(AudioPipeline::with_pool(stats.clone(), false, pool.clone())),
            stats,
            pool,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, Endianness, IdentityProcessor, PcmAudio, PcmFormat, Processor, EncodedAudioFormat};

    #[test]
    fn test_nested_pipeline() {
        let mut inner = AudioPipeline::new_standalone(true);
        inner.with(Processor::Identity(IdentityProcessor));

        let mut outer = AudioPipeline::new_standalone(true);
        outer.with(Processor::Pipeline(Box::new(inner)));

        let spec = EncodedAudioFormat::internal_format().spec();
        let pcm = PcmAudio::new(spec, 480);
        let audio = Audio::from_pcm(&pcm).unwrap();

        let result = outer.process_stream(audio).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_predefined_pipelines() {
        let spec_44100_stereo = AudioFormat {
            sample_rate: 44100,
            channels: 2,
            sample_format: PcmFormat::U32(Endianness::Little),
        };

        // Test TO_INTERNAL
        let internal_format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let stats = Arc::new(LatencyStats::default());
        let to_mono = AudioPipeline::new_input_pipeline(stats.clone(), spec_44100_stereo.clone(), internal_format).unwrap();
        assert!(!to_mono.is_empty());

        // Test FROM_INTERNAL
        let from_mono = AudioPipeline::new_output_pipeline(stats, internal_format, spec_44100_stereo).unwrap();
        assert!(!from_mono.is_empty());
    }
}
