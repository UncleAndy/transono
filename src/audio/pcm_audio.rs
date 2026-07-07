use symphonia::core::audio::{AudioSpec, Channels};

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
    pub(crate) fn new(spec: AudioSpec, frames: usize) -> Self {
        let mut channels = Vec::new();
        for _ in 0..spec.channels().count() {
            channels.push(vec![0.0; frames])
        }

        Self {
            spec,
            channels,
        }
    }

    pub(crate) fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    pub(crate) fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub(crate) fn adapter(
        &mut self,
    ) -> PlanarAdapter<f32> {
        PlanarAdapter::new(&mut self.channels)
    }

    pub(crate) fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }

    pub(crate) fn channels_mut(&mut self) -> &mut [Vec<f32>] {
        self.channels.as_mut_slice()
    }

    pub(crate) fn channel(&self, index: usize) -> &[f32] {
        &self.channels[index]
    }

    pub(crate) fn replace_channel(
        &mut self,
        channel: usize,
        samples: &[f32],
    ) {
        debug_assert_eq!(
            samples.len(),
            self.frames(),
        );

        let dst = &mut self.channels[channel];

        dst.clear();
        dst.extend_from_slice(samples);
    }

    /// Добавляет новый канал.
    pub(crate) fn add_channel(
        &mut self,
        samples: &[f32],
        layout: Channels,
    ) {
        self.assert_frames(samples);

        self.channels.push(samples.to_vec());

        self.set_channels(layout);
    }

    /// Удаляет канал.
    pub(crate) fn remove_channel(
        &mut self,
        channel: usize,
        layout: Channels,
    ) -> Vec<f32> {
        let removed = self.channels.remove(channel);

        self.set_channels(layout);

        removed
    }

    /// Вставляет канал в указанную позицию.
    pub(crate) fn insert_channel(
        &mut self,
        channel: usize,
        samples: &[f32],
        layout: Channels,
    ) {
        self.assert_frames(samples);

        self.channels.insert(
            channel,
            samples.to_vec(),
        );

        self.set_channels(layout);
    }

    /// Заменяет все каналы.
    pub(crate) fn replace_channels(
        &mut self,
        channels: Vec<Vec<f32>>,
        layout: Channels,
    ) {
        debug_assert!(
            channels.is_empty()
                || channels
                .iter()
                .all(|c| c.len() == channels[0].len())
        );

        self.channels = channels;

        self.set_channels(layout);
    }

    pub(crate) fn reserve_channels(
        &mut self,
        additional: usize,
    ) {
        self.channels.reserve(additional);
    }

    /// Удаляет все каналы.
    pub(crate) fn clear_channels(
        &mut self,
    ) {
        self.channels.clear();

        self.set_channels(Channels::None);
    }

    pub(crate) fn set_channel_layout(
        &mut self,
        layout: Channels,
    ) {
        self.set_channels(layout)
    }

    fn set_channels(
        &mut self,
        layout: Channels,
    ) {
        self.spec = AudioSpec::new(
            self.spec.rate(),
            layout,
        );
    }

    fn assert_frames(
        &self,
        samples: &[f32],
    ) {
        debug_assert!(
            self.channels.is_empty()
                || samples.len() == self.frames()
        );
    }
}

/// Byte order for PCM encoded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}
