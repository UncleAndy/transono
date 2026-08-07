use symphonia::core::audio::Channels;

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::{CoreError, Result};

/// A DSP processor for converting between mono and stereo audio.
///
/// Supports mono-to-stereo and stereo-to-mono conversions by averaging
/// or duplicating channels.
pub struct ChannelConverter {
    output_channels: Channels,
}

impl ChannelConverter {
    /// Creates a new [`ChannelConverter`] with the target channel layout.
    ///
    /// # Arguments
    ///
    /// * `output_channels` - The target channel configuration (e.g., mono or stereo).
    pub fn new(
        output_channels: Channels,
    ) -> Self {
        Self {
            output_channels,
        }
    }

    fn stereo_to_mono(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = pcm.frames();

        for i in 0..frames {
            let left = pcm.data[i];
            let right = pcm.data[i + frames];
            pcm.data[i] = (left + right) * 0.5;
        }

        pcm.data.truncate(frames);
        pcm.set_channel_layout(self.output_channels.clone());
    }

    fn mono_to_stereo(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = pcm.frames();
        pcm.data.extend_from_within(0..frames);
        pcm.set_channel_layout(self.output_channels.clone());
    }
}

impl DspProcessor for ChannelConverter {
    fn process(
        &mut self,
        pcm: &mut PcmAudio,
    ) -> Result<bool> {

        match (
            pcm.channel_count(),
            self.output_channels.count(),
        ) {
            (1, 1) | (2, 2) => {
                pcm.set_channel_layout(
                    self.output_channels.clone(),
                );
            }

            (2, 1) => {
                self.stereo_to_mono(pcm);
            }

            (1, 2) => {
                self.mono_to_stereo(pcm);
            }

            (from, to) => {
                return Err(CoreError::Internal(
                    format!("unsupported channel conversion: {} -> {}", from, to)
                ));
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{PcmAudio, PcmFormat, Endianness};
    use symphonia::core::audio::{AudioSpec, Channels};

    fn stereo_pcm(ch0: f32, ch1: f32, frames: usize) -> PcmAudio {
        let mut pcm = PcmAudio::new(
            AudioSpec::new(48000, Channels::Discrete(2)),
            frames,
        );
        for i in 0..frames {
            pcm.data[i] = ch0;
            pcm.data[frames + i] = ch1;
        }
        pcm
    }

    fn mono_pcm(v: f32, frames: usize) -> PcmAudio {
        let mut pcm = PcmAudio::new(
            AudioSpec::new(48000, Channels::Discrete(1)),
            frames,
        );
        for i in 0..frames {
            pcm.data[i] = v;
        }
        pcm
    }

    #[test]
    fn test_stereo_to_mono_averages_planar() {
        // Left = 1.0, Right = 0.0 -> mono should be 0.5 (average, not interleaved mix).
        let mut pcm = stereo_pcm(1.0, 0.0, 16);
        let mut conv = ChannelConverter::new(Channels::Discrete(1));
        assert!(conv.process(&mut pcm).unwrap());

        assert_eq!(pcm.channel_count(), 1);
        assert_eq!(pcm.frames(), 16);
        for i in 0..16 {
            assert!((pcm.data[i] - 0.5).abs() < 1e-6, "sample {} = {}", i, pcm.data[i]);
        }
    }

    #[test]
    fn test_stereo_to_mono_preserves_levels() {
        // Left = 0.2, Right = 0.4 -> mono should be 0.3.
        let mut pcm = stereo_pcm(0.2, 0.4, 8);
        let mut conv = ChannelConverter::new(Channels::Discrete(1));
        assert!(conv.process(&mut pcm).unwrap());
        for i in 0..8 {
            assert!((pcm.data[i] - 0.3).abs() < 1e-6, "sample {} = {}", i, pcm.data[i]);
        }
    }

    #[test]
    fn test_mono_to_stereo_duplicates_planar() {
        // Mono 0.7 -> stereo both channels 0.7 (planar: [0.7 x N, 0.7 x N]).
        let mut pcm = mono_pcm(0.7, 12);
        let mut conv = ChannelConverter::new(Channels::Discrete(2));
        assert!(conv.process(&mut pcm).unwrap());

        assert_eq!(pcm.channel_count(), 2);
        assert_eq!(pcm.frames(), 12);
        assert_eq!(pcm.data.len(), 24);
        for i in 0..12 {
            assert!((pcm.data[i] - 0.7).abs() < 1e-6, "ch0[{}] = {}", i, pcm.data[i]);
            assert!((pcm.data[12 + i] - 0.7).abs() < 1e-6, "ch1[{}] = {}", i, pcm.data[12 + i]);
        }
    }

    #[test]
    fn test_stereo_to_stereo_is_passthrough() {
        let mut pcm = stereo_pcm(0.3, 0.9, 10);
        let mut conv = ChannelConverter::new(Channels::Discrete(2));
        assert!(conv.process(&mut pcm).unwrap());
        for i in 0..10 {
            assert!((pcm.data[i] - 0.3).abs() < 1e-6, "ch0[{}] = {}", i, pcm.data[i]);
            assert!((pcm.data[10 + i] - 0.9).abs() < 1e-6, "ch1[{}] = {}", i, pcm.data[10 + i]);
        }
    }
}
