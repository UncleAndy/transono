use rtrb::{Consumer, Producer, RingBuffer};
use crate::audio::frame::FrameId;
use anyhow::Result;

///! Lock-free очереди передачи владения кадрами.

/// Свободные кадры.
pub type FreeProducer = Producer<FrameId>;
pub type FreeConsumer = Consumer<FrameId>;

/// Кадры, готовые к обработке.
pub type FilledProducer = Producer<FrameId>;
pub type FilledConsumer = Consumer<FrameId>;

pub struct FrameQueues {
    pub free_tx: FreeProducer,
    pub free_rx: FreeConsumer,

    pub filled_tx: FilledProducer,
    pub filled_rx: FilledConsumer,
}

impl FrameQueues {
    pub fn new(frame_count: usize) -> Result<Self> {
        let (free_tx, free_rx) = RingBuffer::new(frame_count);
        let (filled_tx, filled_rx) = RingBuffer::new(frame_count);

        Ok(Self {
            free_tx,
            free_rx,
            filled_tx,
            filled_rx,
        })
    }
}
