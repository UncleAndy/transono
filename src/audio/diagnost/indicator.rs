use tokio::sync::mpsc;

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeIndicator {
    pub rms: f32,
    pub peak: f32,
    pub dbfs: f32,
}

pub struct Indicator {
    sender: mpsc::Sender<VolumeIndicator>,
}

impl Indicator {
    pub fn new(sender: mpsc::Sender<VolumeIndicator>) -> Self {
        Self { sender }
    }

    fn measure(pcm: &PcmAudio) -> VolumeIndicator {
        measure_samples(&pcm.data)
    }
}

impl DspProcessor for Indicator {
    fn process(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        let indicator = Self::measure(pcm);

        let _ = self.sender.try_send(indicator);

        Ok(true)
    }
}

fn measure_samples(samples: &[f32]) -> VolumeIndicator {
    if samples.is_empty() {
        return VolumeIndicator {
            rms: 0.0,
            peak: 0.0,
            dbfs: f32::NEG_INFINITY,
        };
    }

    let mut sum_squares = 0.0;
    let mut peak = 0.0;

    for &sample in samples {
        let abs = sample.abs();
        sum_squares += sample * sample;

        if abs > peak {
            peak = abs;
        }
    }

    let rms = (sum_squares / samples.len() as f32).sqrt();
    let dbfs = if rms > 0.0 {
        20.0 * rms.log10()
    } else {
        f32::NEG_INFINITY
    };

    VolumeIndicator { rms, peak, dbfs }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::audio::{AudioSpec, Channels, Position};

    #[test]
    fn measure_samples_should_return_zero_values_when_input_is_empty() {
        let indicator = measure_samples(&[]);

        assert_eq!(
            indicator,
            VolumeIndicator {
                rms: 0.0,
                peak: 0.0,
                dbfs: f32::NEG_INFINITY,
            },
        );
    }

    #[test]
    fn measure_samples_should_calculate_rms_peak_and_dbfs_for_block() {
        let samples = [-1.0, -0.5, 0.5, 1.0];

        let indicator = measure_samples(&samples);

        assert!((indicator.rms - 0.790_569_4).abs() < f32::EPSILON);
        assert_eq!(indicator.peak, 1.0);
        assert!((indicator.dbfs - -2.041_2).abs() < 0.000_1);
    }

    #[tokio::test]
    async fn process_should_send_indicator_without_changing_audio() {
        let spec = AudioSpec::new(
            48_000,
            Channels::Positioned(Position::FRONT_LEFT | Position::FRONT_RIGHT),
        );
        let mut pcm = PcmAudio::new(spec, 2);
        pcm.data.copy_from_slice(&[0.25, -0.25, 0.5, -0.5]);
        let original = pcm.data.clone();
        let (sender, mut receiver) = mpsc::channel(1);
        let mut indicator = Indicator::new(sender);

        indicator.process(&mut pcm).unwrap();

        assert_eq!(pcm.data, original);
        assert_eq!(receiver.recv().await.unwrap().peak, 0.5);
    }
}
