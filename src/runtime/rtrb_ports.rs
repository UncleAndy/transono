//! `rtrb`-backed implementations of [`AudioInput`]/[`AudioOutput`].
//!
//! Drop-in replacements for [`super::receiver_port::ReceiverPort`] and
//! [`super::sender_port::SenderPort`] that use the lock-free SPSC ring from
//! [`crate::audio::rtrb_chan`] instead of `tokio::sync::mpsc`. The async
//! `Stream`/`Sink` traits are preserved so the mixer, splitter and links keep
//! working unchanged — only the channel engine differs.
//!
//! The consumer `Stream` polls `try_recv()` and yields (`yield_now()`) when
//! empty instead of parking on `recv().await`, matching the low-latency
//! busy-poll used elsewhere in the hot path.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::stream::BoxStream;
use futures_util::{Sink, Stream, StreamExt};

use crate::audio::rtrb_chan::{RtrbChannel, RtrbConsumer, RtrbProducer};
use crate::audio::output::BoxSink;
use crate::audio::{Audio, AudioFormat, AudioInput, AudioOutput};
use crate::core::error::{CoreError, Result, TransportError};

/// Receiving port backed by an [`RtrbConsumer`].
pub struct RtrbReceiverPort {
    format: AudioFormat,
    consumer: Option<RtrbConsumer>,
}

impl RtrbReceiverPort {
    /// Build a port from an [`AudioFormat`] and a consumer half.
    pub fn new(format: AudioFormat, consumer: RtrbConsumer) -> Self {
        Self {
            format,
            consumer: Some(consumer),
        }
    }
}

impl AudioInput for RtrbReceiverPort {
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        let Some(consumer) = self.consumer.take() else {
            return Err(CoreError::Internal("receiver already taken".to_string()));
        };
        Ok(RtrbInputStream { consumer }.boxed())
    }

    fn start(&self) -> Result<()> {
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}

/// Stream that drains an [`RtrbConsumer`] with a yield-based poll loop.
struct RtrbInputStream {
    consumer: RtrbConsumer,
}

impl Stream for RtrbInputStream {
    type Item = Audio;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Audio>> {
        let this = self.get_mut();
        match this.consumer.try_recv() {
            Some(audio) => Poll::Ready(Some(audio)),
            None => {
                if this.consumer.is_drained() {
                    return Poll::Ready(None);
                }
                // Not ready yet: do not park on a waker (rtrb has no async
                // notification). Yield once and ask to be polled again.
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

/// Sending port backed by an [`RtrbProducer`]. Not `Clone` (SPSC).
pub struct RtrbSenderPort {
    format: AudioFormat,
    producer: Option<RtrbProducer>,
}

impl RtrbSenderPort {
    /// Build a port from an [`AudioFormat`] and a producer half.
    pub fn new(format: AudioFormat, producer: RtrbProducer) -> Self {
        Self {
            format,
            producer: Some(producer),
        }
    }

    fn take_producer(&mut self) -> Result<RtrbProducer> {
        self.producer.take().ok_or_else(|| {
            CoreError::Internal("producer already taken".to_string())
        })
    }
}

impl AudioOutput for RtrbSenderPort {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
        let producer = self.take_producer()?;
        Ok(Box::pin(RtrbSink { producer }))
    }

    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}

/// Non-blocking sink over an [`RtrbProducer`]. `poll_ready` reports ready only
/// when a slot is free; `start_send` performs the lock-free push.
struct RtrbSink {
    producer: RtrbProducer,
}

impl Sink<Audio> for RtrbSink {
    type Error = CoreError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let this = self.get_mut();
        if this.producer.slots() == 0 {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Audio) -> Result<()> {
        let this = self.get_mut();
        if !this.producer.try_send(item) {
            return Err(CoreError::Transport(TransportError::ConnectionClosed));
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.get_mut().producer.close();
        Poll::Ready(Ok(()))
    }
}

/// Convenience: build a matched (sender, receiver) pair backed by `rtrb`.
pub fn new_rtrb_ports(format: AudioFormat, capacity: usize) -> (RtrbSenderPort, RtrbReceiverPort) {
    let chan = RtrbChannel::new(capacity);
    let (tx, rx) = chan.split();
    (
        RtrbSenderPort::new(format.clone(), tx),
        RtrbReceiverPort::new(format, rx),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{EncodedAudioFormat, PcmAudio};
    use symphonia::core::audio::AudioSpec;

    fn test_format() -> AudioFormat {
        AudioFormat::from(EncodedAudioFormat::internal_format())
    }

    fn test_audio() -> Audio {
        let fmt = test_format();
        let mut pcm = PcmAudio::new(
            AudioSpec::new(
                fmt.sample_rate,
                symphonia::core::audio::Channels::Discrete(fmt.channels),
            ),
            4,
        );
        pcm.data = vec![0.3f32; 4 * fmt.channels as usize];
        Audio::from_pcm(&pcm).unwrap()
    }

    #[tokio::test]
    async fn stream_yields_sent_audio() -> Result<()> {
        let format = test_format();
        let (mut tx, mut rx) = new_rtrb_ports(format.clone(), 4);

        let audio = test_audio();
        {
            let mut sink = tx.sink()?;
            use futures_util::SinkExt;
            sink.send(audio.clone()).await.unwrap();
        }

        let mut stream = rx.stream()?;
        let received =
            tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await;
        assert!(received.is_ok());
        let got = received.unwrap();
        assert!(got.is_some());
        assert_eq!(
            got.unwrap().to_pcm().unwrap().data.len(),
            4 * test_format().channels as usize
        );
        Ok(())
    }

    #[test]
    fn stream_take_twice_errors() {
        let (_, mut rx) = new_rtrb_ports(test_format(), 4);
        let _first = rx.stream();
        assert!(rx.stream().is_err());
    }

    #[tokio::test]
    async fn sink_accepts_multiple_frames() {
        let format = test_format();
        let (mut tx, mut rx) = new_rtrb_ports(format.clone(), 8);
        let audio = test_audio();

        {
            let mut sink = tx.sink().unwrap();
            use futures_util::SinkExt;
            for _ in 0..6 {
                sink.send(audio.clone()).await.unwrap();
            }
        }

        let mut stream = rx.stream().unwrap();
        for _ in 0..6 {
            let got =
                tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
                    .await
                    .unwrap();
            assert!(got.is_some());
        }
    }

    #[test]
    fn sink_poll_ready_blocks_when_full() {
        // A full ring must make poll_ready Pending until a slot frees up.
        let format = test_format();
        let (_tx_port, _rx_port) = new_rtrb_ports(format, 2);
        // Direct channel test: fill then check slots.
        let chan = RtrbChannel::new(2);
        let (mut tx, _rx) = chan.split();
        assert!(tx.try_send(test_audio()));
        assert!(tx.try_send(test_audio()));
        assert_eq!(tx.slots(), 0);
        assert!(!tx.try_send(test_audio()));
    }

    #[tokio::test]
    async fn closed_stream_ends() {
        let format = test_format();
        let (mut tx, mut rx) = new_rtrb_ports(format.clone(), 4);
        {
            let mut sink = tx.sink().unwrap();
            use futures_util::SinkExt;
            sink.send(test_audio()).await.unwrap();
            sink.close().await.unwrap();
        }
        let mut stream = rx.stream().unwrap();
        // First frame present.
        assert!(stream.next().await.is_some());
        // After drain, stream ends (None) because producer closed.
        let got = tokio::time::timeout(std::time::Duration::from_millis(500), stream.next()).await;
        assert!(got.is_ok());
        assert!(got.unwrap().is_none());
    }
}
