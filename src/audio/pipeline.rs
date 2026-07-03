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

const PROCESSOR_CHUNK: usize = 2400;

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
            let mut playback = Vec::<f32>::new();
            let mut resampler = Resampler::new().unwrap();
            let mut processor_pcm = Vec::<i16>::new();
            let mut input_accumulator = Vec::<i16>::new();

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

                if let Err(err) = resampler.in_processor(
                    &playback,
                    &mut processor_pcm,
                ) {
                    eprintln!("Resampler: {err}");
                    let _ = input.release(frame_id);
                    continue;
                }

                input_accumulator.extend_from_slice(&processor_pcm);

                if input_accumulator.len() < PROCESSOR_CHUNK {
                    let _ = input.release(frame_id);
                    continue;
                }

                if let Err(err) = processor.push_audio(
                    &input_accumulator,
                ) {
                    eprintln!("Processor: {err}");
                    input_accumulator.clear();
                    let _ = input.release(frame_id);
                    continue;
                }

                input_accumulator.clear();

                loop {
                    let chunk = match processor.poll_audio() {
                        Ok(Some(chunk)) => chunk,
                        Ok(None) => break,
                        Err(err) => {
                            eprintln!("Processor: {err}");
                            break;
                        }
                    };

                    playback.clear();

                    if let Err(err) = resampler.out_processor(
                        &chunk,
                        &mut playback,
                    ) {
                        eprintln!("Resampler: {err}");
                        continue;
                    }

                    let mut offset = 0;

                    while offset < playback.len() {
                        let end = (offset + crate::audio::frame::FRAME_CAPACITY)
                            .min(playback.len());

                        let _ = output.send(&playback[offset..end]);

                        offset = end;
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
