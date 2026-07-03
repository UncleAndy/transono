use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use anyhow::Result;

use crate::audio::processor::AudioProcessor;

pub struct AudioPipeline {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AudioPipeline {
    pub fn new(
        mut processor: Box<dyn AudioProcessor>,
    ) -> Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();

        let thread = thread::spawn(move || {
            let mut output_chunks = Vec::<Vec<i16>>::new();

            while thread_running.load(Ordering::Acquire) {
                output_chunks.clear();

                // Следующим коммитом здесь будет:
                //
                // processor.process(input, &mut output_chunks)?;
                //
                // Пока пайплайн только живёт.

                thread::yield_now();
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
