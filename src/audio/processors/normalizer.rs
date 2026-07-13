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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::pcm_audio::PcmAudio;
    use symphonia::core::audio::{AudioSpec, Channels};

    fn create_test_pcm(channels: usize, frames: usize, data: Vec<f32>) -> PcmAudio {
        let internal_spec = crate::audio::EncodedAudioFormat::internal_format().spec();
        let spec = AudioSpec::new(internal_spec.rate(), if channels == 1 {
            Channels::Positioned(symphonia::core::audio::Position::FRONT_CENTER)
        } else if channels == 2 {
            Channels::Positioned(symphonia::core::audio::Position::FRONT_LEFT | symphonia::core::audio::Position::FRONT_RIGHT)
        } else {
            Channels::Positioned(symphonia::core::audio::Position::FRONT_CENTER) // Fallback for tests
        });
        
        let mut pcm = PcmAudio::new(spec, frames);
        pcm.data = data;
        pcm
    }

    #[test]
    fn test_peak_normalization_basic() {
        // Input: max peak is 0.5. Target: 1.0. Gain should be 2.0.
        let data = vec![0.1, 0.5, -0.3, 0.2];
        let mut pcm = create_test_pcm(1, 4, data);
        
        let config = NormalizerConfig {
            normalization_type: NormalizationType::Peak,
            target_level: 1.0,
        };
        let mut normalizer = Normalizer::new(config);
        
        normalizer.process(&mut pcm).unwrap();
        
        assert_eq!(pcm.data[1], 1.0);
        assert_eq!(pcm.data[0], 0.2);
        assert_eq!(pcm.data[2], -0.6);
    }

    #[test]
    fn test_peak_normalization_stereo_preservation() {
        // Left peak: 0.1, Right peak: 0.5. Target: 1.0.
        // Global gain should be 1.0 / 0.5 = 2.0.
        // Left should become 0.2, Right should become 1.0.
        let data = vec![
            0.1, 0.0, // Left channel
            0.5, 0.0  // Right channel
        ];
        let mut pcm = create_test_pcm(2, 2, data);
        
        let config = NormalizerConfig {
            normalization_type: NormalizationType::Peak,
            target_level: 1.0,
        };
        let mut normalizer = Normalizer::new(config);
        
        normalizer.process(&mut pcm).unwrap();
        
        assert_eq!(pcm.data[0], 0.2);
        assert_eq!(pcm.data[2], 1.0);
    }

    #[test]
    fn test_rms_normalization_basic() {
        // Input: [0.5, 0.5]. RMS = sqrt((0.25 + 0.25)/2) = 0.5.
        // Target: 1.0. Gain should be 2.0.
        let data = vec![0.5, 0.5];
        let mut pcm = create_test_pcm(1, 2, data);
        
        let config = NormalizerConfig {
            normalization_type: NormalizationType::Rms,
            target_level: 1.0,
        };
        let mut normalizer = Normalizer::new(config);
        
        normalizer.process(&mut pcm).unwrap();
        
        assert_eq!(pcm.data[0], 1.0);
        assert_eq!(pcm.data[1], 1.0);
    }

    #[test]
    fn test_rms_normalization_complex() {
        // Input: [1.0, 0.0]. RMS = sqrt((1+0)/2) = sqrt(0.5) ≈ 0.7071.
        // Target: 0.7071. Gain should be 1.0.
        let data = vec![1.0, 0.0];
        let mut pcm = create_test_pcm(1, 2, data);
        
        let config = NormalizerConfig {
            normalization_type: NormalizationType::Rms,
            target_level: 0.70710678,
        };
        let mut normalizer = Normalizer::new(config);
        
        normalizer.process(&mut pcm).unwrap();
        
        assert!((pcm.data[0] - 1.0).abs() < 1e-6);
        assert!((pcm.data[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalization_silence() {
        let data = vec![0.0, 0.0, 0.0];
        let mut pcm = create_test_pcm(1, 3, data);
        
        let config = NormalizerConfig {
            normalization_type: NormalizationType::Peak,
            target_level: 1.0,
        };
        let mut normalizer = Normalizer::new(config);
        
        normalizer.process(&mut pcm).unwrap();
        
        // Should remain silence, no NaN or Inf
        assert_eq!(pcm.data, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_empty_buffer() {
        let data = vec![];
        let mut pcm = create_test_pcm(1, 0, data);
        
        let config = NormalizerConfig::default();
        let mut normalizer = Normalizer::new(config);
        
        let result = normalizer.process(&mut pcm);
        assert!(result.is_ok());
    }
}
