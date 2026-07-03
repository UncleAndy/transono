use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use anyhow::Result;
use crate::audio::audio_buffer::{FrameConsumer, FrameProducer};
use crate::audio::processor::AudioProcessor;

pub struct AudioPipeline {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AudioPipeline {
    pub fn new(
        mut input: FrameConsumer,
        mut output: FrameProducer,
        mut processor: Box<dyn AudioProcessor>,
    ) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();

        let thread = thread::spawn(move || {
            let mut input_pcm = Vec::<i16>::new();
            let mut output_chunks = Vec::<Vec<i16>>::new();
            let mut playback = Vec::<f32>::new();

            while thread_running.load(Ordering::Acquire) {
                let Some(frame_id) = input.receive() else {
                    thread::yield_now();
                    continue;
                };

                input_pcm.clear();

                input.read(frame_id, |frame| {
                    if input_pcm.capacity() < frame.len {
                        input_pcm.reserve(frame.len - input_pcm.capacity());
                    }

                    for &sample in &frame.samples[..frame.len] {
                        input_pcm.push(
                            (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
                        );
                    }
                });

                output_chunks.clear();

                if processor.process(&input_pcm, &mut output_chunks).is_ok() {
                    for chunk in &output_chunks {
                        if let Some(id) = output.acquire() {
                            playback.clear();

                            if playback.capacity() < chunk.len() {
                                playback.reserve(chunk.len() - playback.capacity());
                            }
                            
                            for &s in chunk {
                                playback.push(s as f32 / i16::MAX as f32);
                            }

                            if output.write(id, &playback) {
                                let _ = output.commit(id);
                            }
                        }
                    }
                }

                let _ = input.release(frame_id);
            }
        });

        Ok(Self {
            running,
            thread: Some(thread),
        })
    }

    pub fn stop(&mut self) {
        self.running
            .store(false, Ordering::Release);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for AudioPipeline {
    fn drop(&mut self) {
        self.stop();
    }
}
