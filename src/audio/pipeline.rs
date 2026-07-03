use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;

use crate::{
    audio::resampler::Resampler,
    openai::worker::OpenAiWorker,
};

pub struct AudioPipeline {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AudioPipeline {
    pub fn new(
        api_key: String,
        instructions: String,
    ) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));

        let thread_running = running.clone();

        let thread = thread::spawn(move || {
            let mut worker = match OpenAiWorker::connect(
                &api_key,
                &instructions,
            ) {
                Ok(worker) => worker,
                Err(err) => {
                    eprintln!("OpenAI: {err:?}");
                    return;
                }
            };

            let mut resampler = match Resampler::new() {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("Resampler: {err:?}");
                    return;
                }
            };

            let mut input_pcm = Vec::<i16>::new();
            let mut output_pcm = Vec::<i16>::new();
            let mut playback = Vec::<f32>::new();

            while thread_running.load(Ordering::Acquire) {
                //
                // Следующий этап:
                //
                // capture frame
                //      ↓
                // resampler.capture_to_openai(...)
                //      ↓
                // worker.append_audio(...)
                //
                // Пока просто проверяем получение ответа.
                //

                match worker.next_audio() {
                    Ok(Some(chunk)) => {
                        output_pcm.extend(chunk);

                        if resampler
                            .openai_to_playback(
                                &output_pcm,
                                &mut playback,
                            )
                            .is_ok()
                        {
                            output_pcm.clear();

                            //
                            // Следующий коммит:
                            // отправляем playback
                            // в AudioBuffer.
                            //
                        }
                    }

                    Ok(None) => {}

                    Err(err) => {
                        eprintln!("Realtime: {err:?}");
                        thread::sleep(Duration::from_millis(100));
                    }
                }

                let _ = &mut input_pcm;
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
