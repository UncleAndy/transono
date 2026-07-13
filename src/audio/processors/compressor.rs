use crate::audio::processor::DspProcessor;
use crate::audio::pcm_audio::PcmAudio;
use crate::core::error::Result;

/// Audio Compressor (Dynamic Range Compressor)
///
/// Reduces the volume of signals that exceed a certain threshold,
/// effectively narrowing the dynamic range.
pub struct Compressor {
    /// Level (in dB) above which compression starts.
    pub threshold: f32,
    /// Amount of gain reduction (e.g., 4.0 for 4:1).
    pub ratio: f32,
    /// How quickly the compressor reacts to peaks (in seconds).
    pub attack_time: f32,
    /// How quickly the compressor returns to normal gain (in seconds).
    pub release_time: f32,
    /// Gain added after compression to restore overall volume (in dB).
    pub makeup_gain: f32,

    // Internal state for gain smoothing
    envelope: Vec<f32>,
}

pub struct CompressorConfig {
    pub threshold: f32,
    pub ratio: f32,
    pub attack_time: f32,
    pub release_time: f32,
    pub makeup_gain: f32,
}

#[allow(unused)]
static NATURAL_VOICE: CompressorConfig = CompressorConfig {
    threshold: -20.0,
    ratio: 3.0,
    attack_time: 0.01,
    release_time: 0.1,
    makeup_gain: 4.0,
};

#[allow(unused)]
static TIGHT_CONTROL: CompressorConfig = CompressorConfig {
    threshold: -24.0,
    ratio: 4.0,
    attack_time: 0.005, // 5ms
    release_time: 0.06,  // 60ms
    makeup_gain: 6.0,
};

impl Compressor {
    pub fn new(config: CompressorConfig) -> Self {
        Self {
            threshold: config.threshold,
            ratio: config.ratio,
            attack_time: config.attack_time,
            release_time: config.release_time,
            makeup_gain: config.makeup_gain,
            envelope: Vec::new(),
        }
    }

    fn db_to_linear(db: f32) -> f32 {
        10.0f32.powf(db / 20.0)
    }

    fn linear_to_db(linear: f32) -> f32 {
        20.0 * linear.log10().max(-100.0) // Clamp to avoid -inf
    }
}

impl DspProcessor for Compressor {
    fn process(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        let sample_rate = pcm.spec.rate() as f32;
        let channels = pcm.channel_count();
        
        if self.envelope.len() != channels {
            self.envelope.resize(channels, 0.0);
        }

        let makeup_gain_linear = Self::db_to_linear(self.makeup_gain);
        
        // Time constants for attack and release
        let attack_coeff = (-1.0 / (sample_rate * self.attack_time)).exp();
        let release_coeff = (-1.0 / (sample_rate * self.release_time)).exp();

        for chan in 0..channels {
            let samples = pcm.channel_mut(chan);
            let env = &mut self.envelope[chan];

            for sample in samples.iter_mut() {
                // 1. Peak detection (Rectification)
                let abs_sample = sample.abs();
                
                // 2. Envelope following (Attack/Release smoothing)
                let coeff = if abs_sample > *env {
                    attack_coeff
                } else {
                    release_coeff
                };
                *env = coeff * (*env) + (1.0 - coeff) * abs_sample;

                // 3. Gain computation
                let env_db = Self::linear_to_db(*env);
                let reduction_db = if env_db > self.threshold {
                    (self.threshold - env_db) * (1.0 - 1.0 / self.ratio)
                } else {
                    0.0
                };

                let gain = Self::db_to_linear(reduction_db);
                
                // 4. Apply gain and makeup gain
                *sample *= gain * makeup_gain_linear;
            }
        }

        Ok(true)
    }
}
