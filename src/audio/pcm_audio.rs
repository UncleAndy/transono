use symphonia::core::audio::AudioSpec;

use crate::audio::PlanarAdapter;

/// Internal DSP representation.
///
/// The library supports arbitrary sample formats through `Audio`,
/// but all built-in DSP processors currently operate on `f32`.
pub(crate) struct PcmAudio {
    pub spec: AudioSpec,
    pub channels: Vec<Vec<f32>>, // Один Vec на канал.
}

impl PcmAudio {
    pub fn new(spec: AudioSpec, frames: usize) -> Self {
        let mut channels = Vec::new();
        for _ in 0..spec.channels().count() {
            channels.push(vec![0.0; frames])
        }

        Self {
            spec,
            channels,
        }
    }

    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub fn adapter(
        &mut self,
    ) -> PlanarAdapter<f32> {
        PlanarAdapter::new(&mut self.channels)
    }
    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }
    pub fn channels_mut(&mut self) -> &mut [Vec<f32>] {
        self.channels.as_mut_slice()
    }
    pub fn channel(&self, index: usize) -> &[f32] {
        &self.channels[index]
    }
    pub fn replace_channel(
        &mut self,
        channel: usize,
        samples: &[f32],
    ) {
        let dst = &mut self.channels[channel];

        dst.clear();
        dst.extend_from_slice(samples);
    }
}

/// Byte order for PCM encoded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}
