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
    pub capture_timestamp: Instant,
    pub processing_timestamp: Instant,
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
            // При изменении количества каналов планарная структура меняется.
            // Пока просто пересоздаем данные.
            self.data.resize(frames * channels, 0.0);
            self.data.fill(0.0);
        } else if frames != old_frames {
            if frames > old_frames {
                // Увеличиваем: сначала ресайз, потом сдвиг в конец
                self.data.resize(frames * channels, 0.0);
                if channels > 1 && old_frames > 0 {
                    for i in (1..channels).rev() {
                        let old_start = i * old_frames;
                        let new_start = i * frames;
                        self.data.copy_within(old_start..old_start + old_frames, new_start);
                        // Обнуляем старое место (опционально, так как там будут данные следующего канала)
                        self.data[old_start..new_start].fill(0.0);
                    }
                }
            } else {
                // Уменьшаем: сначала сдвиг в начало, потом ресайз
                if channels > 1 && frames > 0 {
                    for i in 1..channels {
                        let old_start = i * old_frames;
                        let new_start = i * frames;
                        self.data.copy_within(old_start..old_start + frames, new_start);
                    }
                }
                self.data.resize(frames * channels, 0.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::audio::Position;

    #[test]
    fn test_pcm_audio_resize_stereo_shrink() {
        let spec = AudioSpec::new(44100, Channels::Positioned(Position::FRONT_LEFT | Position::FRONT_RIGHT));
        let mut pcm = PcmAudio::new(spec, 100);
        
        // Заполним данными
        for i in 0..100 {
            pcm.data[i] = 1.0;     // Left
            pcm.data[100 + i] = 2.0; // Right
        }
        
        // Уменьшаем до 50 фреймов
        pcm.resize(50, 2);
        
        assert_eq!(pcm.frames(), 50);
        assert_eq!(pcm.data.len(), 100);
        
        for i in 0..50 {
            assert_eq!(pcm.data[i], 1.0, "Left channel at {}", i);
            assert_eq!(pcm.data[50 + i], 2.0, "Right channel at {}", i);
        }
    }

    #[test]
    fn test_pcm_audio_resize_stereo_grow() {
        let spec = AudioSpec::new(44100, Channels::Positioned(Position::FRONT_LEFT | Position::FRONT_RIGHT));
        let mut pcm = PcmAudio::new(spec, 50);
        
        // Заполним данными
        for i in 0..50 {
            pcm.data[i] = 1.0;
            pcm.data[50 + i] = 2.0;
        }
        
        // Увеличиваем до 100 фреймов
        pcm.resize(100, 2);
        
        assert_eq!(pcm.frames(), 100);
        assert_eq!(pcm.data.len(), 200);
        
        for i in 0..50 {
            assert_eq!(pcm.data[i], 1.0);
            assert_eq!(pcm.data[100 + i], 2.0);
        }
    }
}
