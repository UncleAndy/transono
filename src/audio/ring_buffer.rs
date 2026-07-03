//! Обертка над rtrb для передачи FrameId между потоками.

use anyhow::{Context, Result};

use rtrb::{Consumer, Producer, RingBuffer};

use crate::audio::frame::FrameId;

pub type FrameProducer = Producer<FrameId>;
pub type FrameConsumer = Consumer<FrameId>;

pub fn create(capacity: usize) -> Result<(FrameProducer, FrameConsumer)> {
    let (producer, consumer) = RingBuffer::new(capacity);

    Ok((producer, consumer))
}
