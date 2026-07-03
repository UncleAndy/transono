//! Lock-free обмен аудиокадрами между потоками.

use std::sync::Arc;

use anyhow::{Result};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::audio::{
    frame::{AudioFrame, FrameId},
    frame_pool::FramePool,
};

pub struct FrameProducer {
    pool: Arc<FramePool>,
    free: Consumer<FrameId>,
    filled: Producer<FrameId>,
}

pub struct FrameConsumer {
    pool: Arc<FramePool>,
    free: Producer<FrameId>,
    filled: Consumer<FrameId>,
}

pub struct AudioBuffer;

impl AudioBuffer {
    pub fn new(frame_count: usize) -> Result<(FrameProducer, FrameConsumer)> {
        let pool = Arc::new(FramePool::new(frame_count));

        let (mut free_tx, free_rx) = RingBuffer::<FrameId>::new(frame_count);
        let (filled_tx, filled_rx) = RingBuffer::<FrameId>::new(frame_count);

        for id in 0..frame_count {
            free_tx
                .push(id as FrameId)
                .map_err(|_| anyhow::anyhow!("failed to initialize free queue"))?;
        }

        Ok((
            FrameProducer {
                pool: Arc::clone(&pool),
                free: free_rx,
                filled: filled_tx,
            },
            FrameConsumer {
                pool,
                free: free_tx,
                filled: filled_rx,
            },
        ))
    }
}

impl FrameProducer {
    #[inline(always)]
    pub fn acquire(&mut self) -> Option<FrameId> {
        self.free.pop().ok()
    }

    #[inline(always)]
    pub fn write(
        &self,
        id: FrameId,
        data: &[f32],
    ) -> bool {
        let frame = self.pool.get_mut(id);

        if data.len() > frame.samples.len() {
            return false;
        }

        frame.len = data.len();
        frame.samples[..data.len()].copy_from_slice(data);

        true
    }

    #[inline(always)]
    pub fn commit(&mut self, id: FrameId) -> Result<()> {
        self.filled
            .push(id)
            .map_err(|_| anyhow::anyhow!("filled queue overflow"))
    }

    pub fn send(
        &mut self,
        data: &[f32],
    ) -> bool {
        let Some(id) = self.acquire() else {
            eprintln!("PLAYBACK BUFFER UNDERRUN");
            return false;
        };

        assert!(self.write(id, data), "frame too large");

        self.commit(id)
            .expect("filled queue overflow");

        true
    }
}

impl FrameConsumer {
    #[inline(always)]
    pub fn receive(&mut self) -> Option<FrameId> {
        self.filled.pop().ok()
    }

    #[inline(always)]
    pub fn read<R>(
        &self,
        id: FrameId,
        f: impl FnOnce(&AudioFrame) -> R,
    ) -> R {
        let frame = self.pool.get(id);
        f(frame)
    }

    #[inline(always)]
    pub fn release(&mut self, id: FrameId) -> Result<()> {
        self.pool.get_mut(id).clear();

        self.free
            .push(id)
            .map_err(|_| anyhow::anyhow!("free queue overflow"))
    }

    #[inline(always)]
    pub fn read_frame(
        &self,
        id: FrameId,
        offset: &mut usize,
        output: &mut [f32],
    ) -> bool {
        self.read(id, |frame| {
            let available = frame.len.saturating_sub(*offset);
            let copied = available.min(output.len());

            output[..copied]
                .copy_from_slice(&frame.samples[*offset..*offset + copied]);

            if copied < output.len() {
                output[copied..].fill(0.0);
            }

            *offset += copied;

            *offset >= frame.len
        })
    }
}
