use crate::audio::{Audio, PcmAudio, Processor};
use crate::core::error::{Result, CoreError};
use anyhow::anyhow;
use std::time::{Instant, Duration};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

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
            min_ms: if min == u64::MAX { 0.0 } else { min as f64 / 1000.0 },
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
}

impl LatencyStats {
    pub fn snapshot(&self) -> LatencySnapshot {
        LatencySnapshot {
            input_pipeline: self.input_pipeline.snapshot(),
            input_total: self.input_total.snapshot(),
            output_pipeline: self.output_pipeline.snapshot(),
            output_total: self.output_total.snapshot(),
        }
    }
}

pub struct AudioPipeline {
    processors: Vec<Processor>,
    scratch_pcm: Option<PcmAudio>,
    stats: Arc<LatencyStats>,
    is_input: bool,
}

impl AudioPipeline {
    pub fn new(stats: Arc<LatencyStats>, is_input: bool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats,
            is_input,
        }
    }

    pub fn new_standalone(is_input: bool) -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
            stats: Arc::new(LatencyStats::default()),
            is_input,
        }
    }

    pub fn add(
        &mut self,
        processor: Processor,
    ) -> &mut Self
    {
        self.processors.push(processor);
        self
    }

    pub fn process(
        &mut self,
        audio: Audio,
    ) -> Result<(Audio, Duration)> {
        let start_time = Instant::now();

        if self.processors.is_empty() {
            return Ok((audio, Duration::from_secs(0)));
        }

        let mut current_audio = Some(audio);

        for processor in &mut self.processors {
            if processor.is_audio() {
                let mut audio = if let Some(a) = current_audio.take() {
                    a
                } else {
                    Audio::from_pcm(self.scratch_pcm.as_ref().ok_or_else(|| CoreError::Other(anyhow!("scratch_pcm missing")))? )?
                };
 
                processor.process_audio(&mut audio)?;
                current_audio = Some(audio);
            } else {
                if let Some(audio) = current_audio.take() {
                    if let Some(ref mut scratch) = self.scratch_pcm {
                        audio.to_pcm_into(scratch)?;
                    } else {
                        self.scratch_pcm = Some(audio.to_pcm()?);
                    }
                }
                processor.process_dsp(self.scratch_pcm.as_mut().ok_or_else(|| CoreError::Other(anyhow!("scratch_pcm missing")))? )?;
            }
        }
 
        let result_audio = if let Some(audio) = current_audio {
            audio
        } else {
            Audio::from_pcm(self.scratch_pcm.as_ref().ok_or_else(|| CoreError::Other(anyhow!("scratch_pcm missing")))? )?
        };

        let duration = start_time.elapsed();
        let duration_us = duration.as_micros() as u64;

        if self.is_input {
            self.stats.input_pipeline.update(duration_us);
            self.stats.input_total.update(result_audio.capture_timestamp().elapsed().as_micros() as u64);
        } else {
            self.stats.output_pipeline.update(duration_us);
            self.stats.output_total.update(result_audio.capture_timestamp().elapsed().as_micros() as u64);
        }

        Ok((result_audio, duration))
    }

    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    pub fn clear(&mut self) {
        self.processors.clear()
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new_standalone(true)
    }
}

pub struct Pipelines {
    pub input: AudioPipeline,
    pub output: AudioPipeline,
    pub stats: Arc<LatencyStats>,
}

impl Pipelines {
    pub fn new() -> Self {
        let stats = Arc::new(LatencyStats::default());
        Self {
            input: AudioPipeline::new(stats.clone(), true),
            output: AudioPipeline::new(stats.clone(), false),
            stats,
        }
    }
}
