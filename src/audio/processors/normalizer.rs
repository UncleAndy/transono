use crate::audio::processor::DspProcessor;
use crate::audio::pcm_audio::PcmAudio;
use crate::core::error::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NormalizationType {
    /// Scales the signal so that the highest peak reaches the target level.
    Peak,
    /// Scales the signal so that the average power (RMS) reaches the target level.
    Rms,
}

#[derive(Debug, Clone)]
pub struct NormalizerConfig {
    pub normalization_type: NormalizationType,
    /// Target level (linear scale for Peak, linear RMS for RMS). 
    /// For Peak, 1.0 is the maximum possible value without clipping.
    pub target_level: f32,
}

impl Default for NormalizerConfig {
    fn default() -> Self {
        Self {
            normalization_type: NormalizationType::Peak,
            target_level: 1.0,
        }
    }
}

/// Audio Normalizer
///
/// Adjusts the gain of the audio signal to reach a target peak or RMS level.
pub struct Normalizer {
    pub config: NormalizerConfig,
}

impl Normalizer {
    pub fn new(config: NormalizerConfig) -> Self {
        Self { config }
    }

    fn apply_gain(samples: &mut [f32], gain: f32) {
        for sample in samples.iter_mut() {
            *sample *= gain;
        }
    }
}

impl DspProcessor for Normalizer {
    fn process(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        let channels = pcm.channel_count();
        
        // We want a global gain across all channels to preserve stereo image
        let mut global_gain = 1.0;
        let mut max_observed_peak = 0.0f32;
        let mut total_sq_sum = 0.0f32;
        let mut total_samples = 0;

        for chan in 0..channels {
            let samples = pcm.channel(chan);
            for &sample in samples {
                let abs_sample = sample.abs();
                if abs_sample > max_observed_peak {
                    max_observed_peak = abs_sample;
                }
                total_sq_sum += sample * sample;
                total_samples += 1;
            }
        }

        if total_samples == 0 {
            return Ok(true);
        }

        match self.config.normalization_type {
            NormalizationType::Peak => {
                if max_observed_peak > 0.0 {
                    global_gain = self.config.target_level / max_observed_peak;
                }
            }
            NormalizationType::Rms => {
                let rms = (total_sq_sum / total_samples as f32).sqrt();
                if rms > 0.0 {
                    global_gain = self.config.target_level / rms;
                }
            }
        }

        // Apply the calculated gain to all channels
        for chan in 0..channels {
            let samples = pcm.channel_mut(chan);
            Self::apply_gain(samples, global_gain);
        }

        Ok(true)
    }
}
