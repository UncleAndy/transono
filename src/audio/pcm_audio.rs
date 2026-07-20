use std::time::Instant;
use symphonia::core::audio::{AudioSpec, Channels};

/// Internal DSP representation.
///
/// The library supports arbitrary sample formats through `Audio`,
/// but all built-in DSP processors currently operate on `f32`.
pub struct PcmAudio {
    /// Audio specification (sample rate, channels).
    pub spec: AudioSpec,
    /// Flat array of samples (planar layout: [chan0][chan1]...).
    pub data: Vec<f32>,
    pub(crate) frames: usize,

    #[allow(unused)]
    pub(crate) sequence: u64,
    /// Timestamp when the audio was captured.
    pub capture_timestamp: Instant,
    /// Timestamp when the audio started processing.
    pub processing_timestamp: Instant,
}

impl PcmAudio {
    /// Creates a new [`PcmAudio`] buffer with the specified spec and frame count.
    ///
    /// # Arguments
    ///
    /// * `spec` - Audio specification (sample rate, channels).
    /// * `frames` - Number of frames to allocate memory for.
    ///
    /// # Returns
    ///
    /// Returns a new instance of [`PcmAudio`] with zeroed data.
    pub fn new(spec: AudioSpec, frames: usize) -> Self {
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

    /// Returns the number of audio frames in the buffer.
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Returns the number of channels.
    pub(crate) fn channel_count(&self) -> usize {
        self.spec.channels().count()
    }

    /// Returns a reference to the samples of a specific channel.
    pub(crate) fn channel(&self, index: usize) -> &[f32] {
        let start = index * self.frames;
        let end = start + self.frames;
        &self.data[start..end]
    }

    /// Returns a mutable reference to the samples of a specific channel.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index of the channel.
    ///
    /// # Panics
    ///
    /// Panics if the `index` is out of bounds for the current channel count.
    pub fn channel_mut(&mut self, index: usize) -> &mut [f32] {
        let start = index * self.frames;
        let end = start + self.frames;
        &mut self.data[start..end]
    }

    /// Sets the channel layout.
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

    /// Resizes the buffer to a new frame and channel count.
    pub(crate) fn resize(&mut self, frames: usize, channels: usize) {
        let old_frames = self.frames;
        let old_channels = self.channel_count();

        if frames == old_frames && channels == old_channels {
            return;
        }

        if channels != old_channels {
            // If the number of channels changes, the planar structure changes.
            // Just resize. fill(0.0) is not needed as data will be overwritten.
            self.data.resize(frames * channels, 0.0);
        } else if frames != old_frames {
            if frames > old_frames {
                // Grow: first resize, then shift to the end
                self.data.resize(frames * channels, 0.0);
                if channels > 1 && old_frames > 0 {
                    for i in (1..channels).rev() {
                        let old_start = i * old_frames;
                        let new_start = i * frames;
                        self.data.copy_within(old_start..old_start + old_frames, new_start);
                        // Zero out old space (optional, as it will be overwritten by the next channel)
                        self.data[old_start..new_start].fill(0.0);
                    }
                }
            } else {
                // Shrink: first shift to the start, then resize
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
    /// Little-endian byte order.
    Little,
    /// Big-endian byte order.
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
