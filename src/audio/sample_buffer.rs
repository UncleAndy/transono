pub struct SampleBuffer<T> {
    data: Vec<T>,
    head: usize,
}

impl<T: Copy> SampleBuffer<T> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            head: 0,
        }
    }

    #[inline(always)]
    pub fn available(&self) -> usize {
        self.data.len() - self.head
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.data.clear();
        self.head = 0;
    }

    #[inline(always)]
    pub fn push(&mut self, input: &[T]) {
        self.data.extend_from_slice(input);
    }

    #[inline(always)]
    pub fn read(&self, count: usize) -> Option<&[T]> {
        if self.available() < count {
            return None;
        }

        Some(&self.data[self.head..self.head + count])
    }

    pub fn consume(&mut self, count: usize) {
        assert!(count <= self.available());

        self.head += count;

        if self.head == self.data.len() {
            self.clear();
            return;
        }

        if self.head > self.data.len() / 2 {
            let tail = self.data.len() - self.head;

            self.data.copy_within(self.head.., 0);

            self.data.truncate(tail);

            self.head = 0;
        }
    }
}

impl<T: Copy> Default for SampleBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

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

    pub fn channel(&self, index: usize) -> Option<&SampleBuffer<T>> {
        self.channels.get(index)
    }

    pub fn channel_mut(&mut self, index: usize) -> Option<&mut SampleBuffer<T>> {
        self.channels.get_mut(index)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_read_discard() {
        let mut buf = SampleBuffer::<i32>::new();

        buf.push(&[1, 2, 3, 4]);

        assert_eq!(buf.available(), 4);
        assert_eq!(buf.read(2), Some(&[1, 2][..]));

        buf.consume(2);

        assert_eq!(buf.read(2), Some(&[3, 4][..]));
        assert_eq!(buf.available(), 2);
    }

    #[test]
    fn compact() {
        let mut buf = SampleBuffer::<i32>::new();

        buf.push(&(0..100).collect::<Vec<_>>());

        buf.consume(80);

        assert_eq!(buf.read(20).unwrap()[0], 80);

        buf.push(&[100, 101]);

        assert_eq!(buf.available(), 22);
    }

    #[test]
    fn clear_after_discard() {
        let mut buf = SampleBuffer::<i32>::new();

        buf.push(&[1, 2, 3]);

        buf.consume(3);

        assert!(buf.is_empty());

        buf.push(&[4]);

        assert_eq!(buf.read(1), Some(&[4][..]));
    }
}
