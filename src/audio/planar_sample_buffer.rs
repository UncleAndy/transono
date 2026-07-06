use crate::audio::SampleBuffer;

pub struct PlanarSampleBuffer<T> {
    channels: Vec<SampleBuffer<T>>,
}

impl<T: Copy> PlanarSampleBuffer<T> {
    pub fn new(channels: usize) -> Self {
        Self {
            channels: (0..channels)
                .map(|_| SampleBuffer::new())
                .collect(),
        }
    }

    #[inline(always)]
    pub fn channels(&self) -> usize {
        self.channels.len()
    }

    #[inline(always)]
    pub fn available(&self) -> usize {
        self.channels
            .first()
            .map_or(0, SampleBuffer::available)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        for channel in &mut self.channels {
            channel.clear();
        }
    }

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

    pub fn push_channel(
        &mut self,
        channel: usize,
        input: &[T],
    ) {
        self.channels[channel].push(input);
    }

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

    #[inline(always)]
    pub fn consume(&mut self, frames: usize) {
        for channel in &mut self.channels {
            channel.consume(frames);
        }
    }

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
