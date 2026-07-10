use std::time::Instant;
use symphonia::core::audio::{AudioSpec, Channels};

/// Internal DSP representation.
///
/// The library supports arbitrary sample formats through `Audio`,
/// but all built-in DSP processors currently operate on `f32`.
pub struct PcmAudio {
    pub spec: AudioSpec,
    pub data: Vec<f32>, // Один плоский массив для всех каналов (планарный: [chan0][chan1]...).
    pub(crate) frames: usize,

    #[allow(unused)]
    pub(crate) sequence: u64,
    #[allow(unused)]
    pub(crate) capture_timestamp: Instant,
    #[allow(unused)]
    pub(crate) processing_timestamp: Instant,
}

impl PcmAudio {
    pub(crate) fn new(spec: AudioSpec, frames: usize) -> Self {
        let channel_count = spec.channels().count();
        let data = vec![0.0; frames * channel_count];

        Self {
            spec,
            data,
            frames,
            sequence: 0,
            capture_timestamp: Instant::now(),
            processing_timestamp: Instant::now(),
        }
    }

    pub(crate) fn frames(&self) -> usize {
        self.frames
    }

    pub(crate) fn channel_count(&self) -> usize {
        self.spec.channels().count()
    }

    pub(crate) fn channel(&self, index: usize) -> &[f32] {
        let start = index * self.frames;
        let end = start + self.frames;
        &self.data[start..end]
    }

    pub(crate) fn channel_mut(&mut self, index: usize) -> &mut [f32] {
        let start = index * self.frames;
        let end = start + self.frames;
        &mut self.data[start..end]
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

    pub(crate) fn resize(&mut self, frames: usize, channels: usize) {
        let old_frames = self.frames;
        let old_channels = self.channel_count();

        if frames == old_frames && channels == old_channels {
            return;
        }

        if channels != old_channels {
            // Если количество каналов меняется, это сложнее.
            // Но обычно мы меняем только фреймы или пересоздаем целиком.
            self.data.resize(frames * channels, 0.0);
            if frames != old_frames {
                 // Тут нужна сложная логика перемещения данных если каналов > 1
                 // Но для простоты пока просто обнулим или переаллоцируем.
                 // В реальности ресемплер вызывает это.
                 if old_channels > 1 && old_frames > 0 {
                      // TODO: корректный ресайз планарных данных
                 }
            }
        } else {
            // Количество каналов то же, меняем фреймы.
            if frames != old_frames {
                self.data.resize(frames * channels, 0.0);
                if channels > 1 && old_frames > 0 {
                    // Перемещаем каналы (кроме первого)
                    for i in (1..channels).rev() {
                        let old_start = i * old_frames;
                        let new_start = i * frames;
                        let count = old_frames.min(frames);
                        self.data.copy_within(old_start..old_start + count, new_start);
                    }
                }
            }
        }

        self.frames = frames;
    }
}

/// Byte order for PCM encoded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Endianness {
    Little,
    Big,
}
