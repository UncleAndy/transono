//! Lock-free SPSC audio channel built on `rtrb` (zero-alloc, zero-copy
//! `try_send`/`try_recv`) with an explicit close flag.
//!
//! Replaces `tokio::sync::mpsc` for the graph's SPSC links (splitter output ->
//! link, link -> link, mixer -> output) so the hot path no longer pays the
//! mpsc allocation + async scheduler cost per frame.
//!
//! `rtrb` 0.3 has no built-in `close()`, so an `Arc<AtomicBool>` carries the
//! "producer is done" signal shared between the two halves.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::audio::Audio;

/// Owns the underlying `rtrb` ring and the shared closed flag.
pub struct RtrbChannel {
    producer: Producer<Audio>,
    consumer: Consumer<Audio>,
    closed: Arc<AtomicBool>,
}

impl RtrbChannel {
    /// Create a channel with the given number of slots (minimum 1).
    pub fn new(capacity: usize) -> Self {
        let (producer, consumer) = RingBuffer::<Audio>::new(capacity.max(1));
        Self {
            producer,
            consumer,
            closed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Split into the producer (send) and consumer (receive) halves.
    pub fn split(self) -> (RtrbProducer, RtrbConsumer) {
        (
            RtrbProducer {
                inner: self.producer,
                closed: self.closed.clone(),
            },
            RtrbConsumer {
                inner: self.consumer,
                closed: self.closed.clone(),
            },
        )
    }
}

/// Sending half of an [`RtrbChannel`]. Single-producer (not `Clone`).
pub struct RtrbProducer {
    inner: Producer<Audio>,
    closed: Arc<AtomicBool>,
}

impl RtrbProducer {
    /// Attempt to enqueue one frame. Returns `false` if the ring is full.
    #[inline]
    pub fn try_send(&mut self, audio: Audio) -> bool {
        self.inner.push(audio).is_ok()
    }

    /// Mark the channel closed: no further sends are accepted and consumers
    /// will drain then stop once the ring empties.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    /// True once [`close`](Self::close) has been called.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Number of free slots remaining.
    pub fn slots(&self) -> usize {
        self.inner.slots()
    }
}

/// Receiving half of an [`RtrbChannel`]. Single-consumer (not `Clone`).
pub struct RtrbConsumer {
    inner: Consumer<Audio>,
    closed: Arc<AtomicBool>,
}

impl RtrbConsumer {
    /// Attempt to dequeue one frame. Returns `None` if the ring is empty.
    #[inline]
    pub fn try_recv(&mut self) -> Option<Audio> {
        self.inner.pop().ok()
    }

    /// True once the producer has called
    /// [`close`](RtrbProducer::close).
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// True when there is no readable frame AND the producer has closed.
    pub fn is_drained(&self) -> bool {
        self.closed.load(Ordering::SeqCst) && self.inner.slots() == 0
    }

    /// Number of readable frames available.
    pub fn slots(&self) -> usize {
        self.inner.slots()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, AudioFormat, EncodedAudioFormat, PcmAudio};
    use symphonia::core::audio::AudioSpec;

    fn test_audio() -> Audio {
        let fmt = AudioFormat::from(EncodedAudioFormat::internal_format());
        let mut pcm = PcmAudio::new(
            AudioSpec::new(
                fmt.sample_rate,
                symphonia::core::audio::Channels::Discrete(fmt.channels),
            ),
            4,
        );
        pcm.data = vec![0.25f32; 4 * fmt.channels as usize];
        Audio::from_pcm(&pcm).unwrap()
    }

    #[test]
    fn round_trip_send_recv() {
        let chan = RtrbChannel::new(4);
        let (mut tx, mut rx) = chan.split();
        assert!(tx.try_send(test_audio()));
        let got = rx.try_recv();
        assert!(got.is_some());
        // internal_format is mono (1 channel) x 4 frames.
        assert_eq!(got.unwrap().to_pcm().unwrap().data.len(), 4 * 1);
    }

    #[test]
    fn recv_empty_when_nothing_sent() {
        let chan = RtrbChannel::new(4);
        let (_tx, mut rx) = chan.split();
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn full_ring_rejects_send() {
        let chan = RtrbChannel::new(2);
        let (mut tx, _rx) = chan.split();
        assert!(tx.try_send(test_audio()));
        assert!(tx.try_send(test_audio()));
        // Third push must fail (capacity 2).
        assert!(!tx.try_send(test_audio()));
    }

    #[test]
    fn close_signals_drained_after_empty() {
        let chan = RtrbChannel::new(2);
        let (mut tx, mut rx) = chan.split();
        tx.try_send(test_audio());
        assert!(!rx.is_drained());
        tx.close();
        // Still has one frame, so not drained yet.
        assert!(!rx.is_drained());
        let _ = rx.try_recv();
        // Empty + closed => drained.
        assert!(rx.is_drained());
        assert!(rx.try_recv().is_none());
    }

    #[test]
    fn closed_flag_visible_on_both_halves() {
        let chan = RtrbChannel::new(2);
        let (tx, rx) = chan.split();
        assert!(!tx.is_closed());
        assert!(!rx.is_closed());
        tx.close();
        assert!(tx.is_closed());
        assert!(rx.is_closed());
    }
}
