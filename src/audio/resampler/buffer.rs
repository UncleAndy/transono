use std::fmt::Debug;

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
    pub fn push(
        &mut self,
        input: &[T],
    ) {
        self.data.extend_from_slice(input);
    }

    #[inline(always)]
    pub fn read(
        &self,
        count: usize,
    ) -> Option<&[T]> {
        if self.available() < count {
            return None;
        }

        Some(&self.data[self.head..self.head + count])
    }

    pub fn consume(
        &mut self,
        count: usize,
    ) {
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
