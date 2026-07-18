//! Lock-free обмен аудиокадрами между потоками.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::core::error::Result;
use rtrb::{Consumer, Producer, RingBuffer};
use symphonia::core::audio::GenericAudioBuffer;
use crate::audio::{frame::{AudioFrame, FrameId}, frame_pool::FramePool, Audio, PcmAudio, FRAME_CAPACITY};

pub struct FrameProducer {
    pool: Arc<FramePool>,
    free: Consumer<FrameId>,
    filled: Producer<FrameId>,
    inner: Arc<AudioBuffer>,
}

pub struct FrameConsumer {
    pool: Arc<FramePool>,
    free: Producer<FrameId>,
    filled: Consumer<FrameId>,
    inner: Arc<AudioBuffer>,
}

pub struct AudioBuffer {
    ready_count: AtomicUsize,
}

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

        let inner = Arc::new(AudioBuffer {
            ready_count: AtomicUsize::new(0),
        });

        Ok((
            FrameProducer {
                pool: Arc::clone(&pool),
                free: free_rx,
                filled: filled_tx,
                inner: inner.clone(),
            },
            FrameConsumer {
                pool,
                free: free_tx,
                filled: filled_rx,
                inner,
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
        let pushed = self.filled
            .push(id)
            .map_err(|_| "filled queue overflow".into());

        if pushed.is_ok() {
            self.inner.ready_count.fetch_add(1, Ordering::SeqCst);
        }

        pushed
    }

    #[inline(always)]
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

    #[inline(always)]
    pub fn send_audio(
        &mut self,
        audio: &Audio,
        scratch: &mut Vec<f32>,
    ) -> Result<bool> {
        let pcm = audio.to_pcm()?;

        Self::write_interleaved(
            &pcm,
            scratch,
        );

        self.send(scratch)
    }

    /// Нет свободных кадров для записи.
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.free.slots() == 0
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.inner.ready_count.load(Ordering::SeqCst) == 0
    }

    #[inline(always)]
    pub fn has_frame(&self) -> bool {
        self.inner.ready_count.load(Ordering::SeqCst) != 0
    }

    /// Есть хотя бы один свободный кадр.
    #[inline(always)]
    pub fn has_free_frame(&self) -> bool {
        self.free.slots() > 0
    }

    #[inline(always)]
    fn write_interleaved(
        pcm: &PcmAudio,
        output: &mut Vec<f32>,
    ) {
        output.clear();

        let channels = pcm.channel_count();
        let frames = pcm.frames();

        output.reserve(frames * channels);

        for frame in 0..frames {
            for channel in 0..channels {
                output.push(
                    pcm.channel(channel)[frame]
                );
            }
        }
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

        self.inner.ready_count.fetch_sub(1, Ordering::SeqCst);

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

    /// Нет готовых кадров.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.inner.ready_count.load(Ordering::SeqCst) == 0
    }

    /// Есть готовый кадр.
    #[inline(always)]
    pub fn has_frame(&self) -> bool {
        self.inner.ready_count.load(Ordering::SeqCst) != 0
    }
}

#[allow(unused)]
pub trait IntoGenericAudioBuffer {
    fn into_generic(self) -> GenericAudioBuffer;
}
