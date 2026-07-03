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
use crate::audio::resampler::Resampler;

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
            let mut resampler = Resampler::new().unwrap();
            let mut processor_pcm = Vec::<i16>::new();

            while thread_running.load(Ordering::Acquire) {
                let Some(frame_id) = input.receive() else {
                    thread::yield_now();
                    continue;
                };

                playback.clear();

                input.read(frame_id, |frame| {
                    playback.clear();

                    if playback.capacity() < frame.len {
                        playback.reserve(frame.len - playback.capacity());
                    }

                    playback.extend_from_slice(&frame.samples[..frame.len]);
                });

                processor_pcm.clear();

                let _ = resampler.in_processor(
                    &playback,
                    &mut processor_pcm,
                );

                output_chunks.clear();

                if processor.process(&processor_pcm, &mut output_chunks).is_ok() {
                    for chunk in &output_chunks {
                        if let Some(id) = output.acquire() {
                            playback.clear();

                            if let Err(err) = resampler.out_processor(
                                chunk,
                                &mut playback,
                            ) {
                                eprintln!("Resampler: {err}");
                                continue;
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
