use crate::audio::SampleBuffer;

/// A buffer for planar audio samples (each channel in its own contiguous memory).
pub struct PlanarSampleBuffer<T> {
    channels: Vec<SampleBuffer<T>>,
}

impl<T: Copy> PlanarSampleBuffer<T> {
    /// Creates a new planar sample buffer with the specified number of channels.
    pub fn new(channels: usize) -> Self {
        Self {
            channels: (0..channels)
                .map(|_| SampleBuffer::new())
                .collect(),
        }
    }

    /// Returns the number of channels in the buffer.
    #[inline(always)]
    pub fn channels(&self) -> usize {
        self.channels.len()
    }

    /// Returns the number of available frames.
    #[inline(always)]
    pub fn available(&self) -> usize {
        self.channels
            .first()
            .map_or(0, SampleBuffer::available)
    }

    /// Returns true if the buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    /// Clears all samples from the buffer.
    #[inline(always)]
    pub fn clear(&mut self) {
        for channel in &mut self.channels {
            channel.clear();
        }
    }

    /// Pushes multiple channels of planar audio into the buffer.
    pub fn push(&mut self, input: &[&[T]]) {
        assert_eq!(
            input.len(),
            self.channels.len(),
            "channel count mismatch",
        );

        if input.is_empty() {
            return;
        }

        let frames = input[0].len();

        debug_assert!(
            input.iter().all(|c| c.len() == frames),
            "all channels must contain the same number of frames",
        );

        for (buffer, samples) in self.channels.iter_mut().zip(input) {
            buffer.push(samples);
        }
    }

    /// Pushes a single channel of audio into the buffer.
    pub fn push_channel(
        &mut self,
        channel: usize,
        input: &[T],
    ) {
        self.channels[channel].push(input);
    }

    /// Reads a slice of samples for a specific channel.
    #[inline(always)]
    pub fn read_channel(
        &self,
        channel: usize,
        frames: usize,
    ) -> Option<&[T]> {
        self.channels
            .get(channel)?
            .read(frames)
    }

    /// Consumes the specified number of frames from all channels.
    #[inline(always)]
    pub fn consume(&mut self, frames: usize) {
        for channel in &mut self.channels {
            channel.consume(frames);
        }
    }

    /// Resizes the buffer to a new number of channels and clears it.
    #[inline(always)]
    pub fn resize(&mut self, channels: usize) {
        self.clear();

        match channels.cmp(&self.channels.len()) {
            std::cmp::Ordering::Greater => {
                self.channels
                    .resize_with(channels, SampleBuffer::new);
            }
            std::cmp::Ordering::Less => {
                self.channels.truncate(channels);
            }
            std::cmp::Ordering::Equal => {}
        }
    }
}

impl<T: Copy> Default for PlanarSampleBuffer<T> {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
        }
    }
}
