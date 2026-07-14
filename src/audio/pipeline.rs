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

#[derive(Debug)]
pub struct LatencyMetric {
    pub min_us: AtomicU64,
    pub max_us: AtomicU64,
    pub sum_us: AtomicU64,
    pub count: AtomicU64,
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
    pub fn update(&self, value_us: u64) {
        self.min_us.fetch_min(value_us, Ordering::Relaxed);
        self.max_us.fetch_max(value_us, Ordering::Relaxed);
        self.sum_us.fetch_add(value_us, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.last_us.store(value_us, Ordering::Relaxed);
    }

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

#[derive(Debug, Default)]
pub struct LatencyStats {
    pub input_pipeline: LatencyMetric,
    pub input_total: LatencyMetric,
    pub output_pipeline: LatencyMetric,
    pub output_total: LatencyMetric,
    pub dropped_input: AtomicU64,
    pub dropped_network: AtomicU64,
    pub dropped_output: AtomicU64,
    pub output_active: AtomicBool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MetricSnapshot {
    pub min_ms: f64,
    pub max_ms: f64,
    pub avg_ms: f64,
    pub last_ms: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LatencySnapshot {
    pub input_pipeline: MetricSnapshot,
    pub input_total: MetricSnapshot,
    pub output_pipeline: MetricSnapshot,
    pub output_total: MetricSnapshot,
    pub dropped_input: u64,
    pub dropped_network: u64,
    pub dropped_output: u64,
}

impl LatencyStats {
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

    pub fn inc_dropped_input(&self) {
        self.dropped_input.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_dropped_network(&self) {
        self.dropped_network.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_dropped_output(&self) {
        if self.output_active.load(Ordering::Relaxed) {
            self.dropped_output.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn set_output_active(&self, active: bool) {
        self.output_active.store(active, Ordering::Relaxed);
    }
}

pub struct AudioPipeline {
    processors: Vec<Processor>,
    scratch_pcm: Option<PcmAudio>,
    stats: Arc<LatencyStats>,
    is_input: bool,
    pool: SharedPcmPool,
}

impl AudioPipeline {
    pub fn new(stats: Arc<LatencyStats>, is_input: bool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats,
            is_input,
            pool: Arc::new(PcmPool::new()),
        }
    }
 
    pub fn with_pool(stats: Arc<LatencyStats>, is_input: bool, pool: SharedPcmPool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats,
            is_input,
            pool,
        }
    }
 
    pub fn new_standalone(is_input: bool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats: Arc::new(LatencyStats::default()),
            is_input,
            pool: Arc::new(PcmPool::new()),
        }
    }

    pub fn with(&mut self, processor: Processor) -> &mut Self {
        self.processors.push(processor);
        self
    }

    /// Создает пайплайн для преобразования входного аудио между форматами.
    pub fn new_input_pipeline(from: AudioFormat, to: AudioFormat) -> Result<Box<Self>> {
        let mut pipeline = Self::new_standalone(true);

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

    /// Создает пайплайн для преобразования выходного аудио между форматами.
    pub fn new_output_pipeline(from: AudioFormat, to: AudioFormat) -> Result<Box<Self>> {
        let mut pipeline = Self::new_standalone(false);

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

    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    pub fn clear(&mut self) {
        self.processors.clear()
    }

    /// Внутренний метод обработки, который можно вызывать из трейтов.
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

pub struct Pipelines {
    pub input: Box<dyn Pipeline>,
    pub output: Box<dyn Pipeline>,
    pub stats: Arc<LatencyStats>,
    pub pool: SharedPcmPool,
}
 
impl Pipelines {
    pub fn new() -> Self {
        Self::with_stats(Arc::new(LatencyStats::default()))
    }
 
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
        let to_mono = AudioPipeline::new_input_pipeline(spec_44100_stereo.clone(), internal_format).unwrap();
        assert!(!to_mono.is_empty());

        // Test FROM_INTERNAL
        let from_mono = AudioPipeline::new_output_pipeline(internal_format, spec_44100_stereo).unwrap();
        assert!(!from_mono.is_empty());
    }
}
