//! Lock-free обмен аудиокадрами между потоками.

use std::sync::Arc;

use anyhow::{Context, Result};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::audio::{
    frame::{AudioFrame, FrameId},
    frame_pool::FramePool,
};

pub struct CaptureSide {
    pool: Arc<FramePool>,
    free: Consumer<FrameId>,
    filled: Producer<FrameId>,
}

pub struct PipelineSide {
    pool: Arc<FramePool>,
    free: Producer<FrameId>,
    filled: Consumer<FrameId>,
}

pub struct AudioBuffer;

impl AudioBuffer {
    pub fn new(frame_count: usize) -> Result<(CaptureSide, PipelineSide)> {
        let pool = Arc::new(FramePool::new(frame_count));

        let (mut free_tx, free_rx) = RingBuffer::<FrameId>::new(frame_count);
        let (filled_tx, filled_rx) = RingBuffer::<FrameId>::new(frame_count);

        for id in 0..frame_count {
            free_tx
                .push(id as FrameId)
                .map_err(|_| anyhow::anyhow!("failed to initialize free queue"))?;
        }

        Ok((
            CaptureSide {
                pool: Arc::clone(&pool),
                free: free_rx,
                filled: filled_tx,
            },
            PipelineSide {
                pool,
                free: free_tx,
                filled: filled_rx,
            },
        ))
    }
}

impl CaptureSide {
    #[inline(always)]
    pub fn acquire(&mut self) -> Option<FrameId> {
        self.free.pop().ok()
    }

    #[inline(always)]
    pub fn frame_mut(&self, id: FrameId) -> &mut AudioFrame {
        self.pool.get_mut(id)
    }

    #[inline(always)]
    pub fn commit(&mut self, id: FrameId) -> Result<()> {
        self.filled
            .push(id)
            .map_err(|_| anyhow::anyhow!("filled queue overflow"))
    }
}

impl PipelineSide {
    #[inline(always)]
    pub fn receive(&mut self) -> Option<FrameId> {
        self.filled.pop().ok()
    }

    #[inline(always)]
    pub fn frame(&self, id: FrameId) -> &AudioFrame {
        self.pool.get(id)
    }

    #[inline(always)]
    pub fn release(&mut self, id: FrameId) -> Result<()> {
        self.pool.get_mut(id).clear();

        self.free
            .push(id)
            .map_err(|_| anyhow::anyhow!("free queue overflow"))
    }
}
