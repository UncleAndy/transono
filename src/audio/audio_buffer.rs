//! Lock-free обмен аудиокадрами между потоками.

use std::sync::Arc;

use crate::core::error::Result;
use rtrb::{Consumer, Producer, RingBuffer};
use symphonia::core::audio::GenericAudioBuffer;
use crate::audio::{
    frame::{AudioFrame, FrameId},
    frame_pool::FramePool,
    FRAME_CAPACITY,
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
                .map_err(|_| "failed to initialize free queue")?;
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
    pub fn write(&self, id: FrameId, data: &[f32]) -> bool {
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
            .map_err(|_| "filled queue overflow".into())
    }
 
    pub fn send(&mut self, data: &[f32]) -> Result<bool> {
        let Some(id) = self.acquire() else {
            return Ok(false);
        };
 
        if !self.write(id, data) {
            return Err(format!(
                "frame too large: {} samples (capacity {})",
                data.len(),
                FRAME_CAPACITY
            ).into());
        }
        self.commit(id)?;
 
        Ok(true)
    }
}

impl FrameConsumer {
    #[inline(always)]
    pub fn receive(&mut self) -> Option<FrameId> {
        self.filled.pop().ok()
    }

    #[inline(always)]
    pub fn read<R>(&self, id: FrameId, f: impl FnOnce(&AudioFrame) -> R) -> R {
        let frame = self.pool.get(id);
        f(frame)
    }

    #[inline(always)]
    pub fn release(&mut self, id: FrameId) -> Result<()> {
        self.pool.get_mut(id).clear();
 
        self.free
            .push(id)
            .map_err(|_| "free queue overflow".into())
    }

    #[inline(always)]
    pub fn read_frame(&self, id: FrameId, offset: &mut usize, output: &mut [f32]) -> bool {
        self.read(id, |frame| {
            let available = frame.len.saturating_sub(*offset);
            let copied = available.min(output.len());

            output[..copied].copy_from_slice(&frame.samples[*offset..*offset + copied]);

            if copied < output.len() {
                output[copied..].fill(0.0);
            }

            *offset += copied;

            *offset >= frame.len
        })
    }

    #[inline(always)]
    pub fn receive_frame(
        &mut self,
        current: &mut Option<FrameId>,
        offset: &mut usize,
        output: &mut [f32],
    ) {
        loop {
            if current.is_none() {
                *current = self.receive();

                if current.is_none() {
                    output.fill(0.0);
                    return;
                }

                *offset = 0;
            }

            let finished = self.read_frame(
                current.unwrap(),
                offset,
                output,
            );

            if finished {
                if let Some(id) = current.take() {
                    let _ = self.release(id);
                }
            }

            return;
        }
    }

    #[inline(always)]
    pub fn fill_buffer(
        &mut self,
        current: &mut Option<FrameId>,
        offset: &mut usize,
        output: &mut [f32],
    ) {
        let mut written = 0;

        while written < output.len() {

            if current.is_none() {
                *current = self.receive();

                if current.is_none() {
                    output[written..].fill(0.0);
                    return;
                }

                *offset = 0;
            }

            let id = current.unwrap();

            let (copied, finished) = self.read(id, |frame| {

                let available = frame.len.saturating_sub(*offset);
                let count = available.min(output.len() - written);

                output[written..written + count]
                    .copy_from_slice(
                        &frame.samples[*offset..*offset + count]
                    );

                (count, *offset + count >= frame.len)
            });

            written += copied;
            *offset += copied;

            if finished {
                *offset = 0;
                if let Some(id) = current.take() {
                    let _ = self.release(id);
                }
            }
        }
    }
}

#[allow(unused)]
pub trait IntoGenericAudioBuffer {
    fn into_generic(self) -> GenericAudioBuffer;
}
