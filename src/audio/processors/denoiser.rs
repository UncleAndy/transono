use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::Result;
use symphonia::core::audio::AudioSpec;

const SIGNAL_FLOOR_DBFS: f32 = -60.0;
const NOISE_FLOOR_ATTACK: f32 = 0.12;
const NOISE_FLOOR_RELEASE: f32 = 0.01;
const GATE_OPEN_DB: f32 = 22.0;
const GATE_CLOSE_DB: f32 = 14.0;
const MAX_ATTENUATION_DB: f32 = 18.0;
const ATTACK_SMOOTHING: f32 = 0.45;
const RELEASE_SMOOTHING: f32 = 0.08;

pub struct Denoiser {
    channels: usize,
    noise_floor_dbfs: f32,
    current_attenuation_db: f32,
}

impl Denoiser {
    pub fn new(spec: AudioSpec) -> Self {
        Self {
            channels: spec.channels().count(),
            noise_floor_dbfs: SIGNAL_FLOOR_DBFS,
            current_attenuation_db: 0.0,
        }
    }

    fn measure_block_dbfs(&self, pcm: &PcmAudio) -> f32 {
        let mut sum_squares = 0.0;
        let mut count = 0usize;

        for channel in 0..pcm.channel_count() {
            for &sample in pcm.channel(channel) {
                sum_squares += sample * sample;
                count += 1;
            }
        }

        if count == 0 {
            return SIGNAL_FLOOR_DBFS;
        }

        let rms = (sum_squares / count as f32).sqrt();
        amplitude_to_dbfs(rms)
    }

    fn estimate_noise_floor(&mut self, block_dbfs: f32, attenuation_db: f32) {
        let block_dbfs = finite_or_floor(block_dbfs);

        if attenuation_db > 0.5 {
            self.noise_floor_dbfs = lerp(self.noise_floor_dbfs, block_dbfs, NOISE_FLOOR_ATTACK);
        } else if block_dbfs < self.noise_floor_dbfs {
            self.noise_floor_dbfs = lerp(self.noise_floor_dbfs, block_dbfs, NOISE_FLOOR_RELEASE);
        }

        self.noise_floor_dbfs = self.noise_floor_dbfs.max(SIGNAL_FLOOR_DBFS);
    }

    fn compute_target_attenuation(&mut self, block_dbfs: f32) -> f32 {
        let block_dbfs = finite_or_floor(block_dbfs);
        let open_threshold = self.noise_floor_dbfs + GATE_OPEN_DB;
        let close_threshold = self.noise_floor_dbfs + GATE_CLOSE_DB;

        let target = if block_dbfs >= open_threshold {
            0.0
        } else if block_dbfs <= close_threshold {
            let diff = (close_threshold - block_dbfs).max(0.0);
            (diff * 1.25).min(MAX_ATTENUATION_DB)
        } else {
            let span = (open_threshold - close_threshold).max(0.001);
            let t = (block_dbfs - close_threshold) / span;
            lerp(MAX_ATTENUATION_DB, 0.0, t.clamp(0.0, 1.0))
        };

        let smoothing = if target < self.current_attenuation_db {
            ATTACK_SMOOTHING
        } else {
            RELEASE_SMOOTHING
        };

        self.current_attenuation_db = lerp(self.current_attenuation_db, target, smoothing);
        self.current_attenuation_db.clamp(0.0, MAX_ATTENUATION_DB)
    }

    fn apply_attenuation(&self, pcm: &mut PcmAudio, attenuation_db: f32) {
        if attenuation_db <= 0.01 {
            return;
        }

        let gain = db_to_gain(-attenuation_db);

        for channel in 0..pcm.channel_count() {
            for sample in pcm.channel_mut(channel) {
                *sample *= gain;
            }
        }
    }
}

impl DspProcessor for Denoiser {
    fn process(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        debug_assert_eq!(self.channels, pcm.channel_count());

        let block_dbfs = self.measure_block_dbfs(pcm);
        let attenuation_db = self.compute_target_attenuation(block_dbfs);
        self.estimate_noise_floor(block_dbfs, attenuation_db);
        self.apply_attenuation(pcm, attenuation_db);

        Ok(true)
    }
}

fn amplitude_to_dbfs(amplitude: f32) -> f32 {
    if amplitude > 0.0 {
        20.0 * amplitude.log10()
    } else {
        SIGNAL_FLOOR_DBFS
    }
}

fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

fn finite_or_floor(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        SIGNAL_FLOOR_DBFS
    }
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::EncodedAudioFormat;

    #[test]
    fn denoiser_should_preserve_loud_signal() {
        let spec = EncodedAudioFormat::internal_format().spec();
        let mut pcm = PcmAudio::new(spec.clone(), 8);
        pcm.data.fill(0.6);
        let original = pcm.data.clone();
        let mut denoiser = Denoiser::new(spec);

        denoiser.process(&mut pcm).unwrap();

        assert!(pcm.data.iter().all(|sample| sample.abs() >= 0.5));
        assert!(
            pcm.data
                .iter()
                .zip(original.iter())
                .all(|(a, b)| *a >= *b * 0.6)
        );
    }

    #[test]
    fn denoiser_should_attenuate_low_noise() {
        let spec = EncodedAudioFormat::internal_format().spec();
        let mut pcm = PcmAudio::new(spec.clone(), 8);
        pcm.data.fill(0.01);
        let mut denoiser = Denoiser::new(spec);

        denoiser.process(&mut pcm).unwrap();

        assert!(pcm.data.iter().all(|sample| sample.abs() < 0.01));
    }
}
