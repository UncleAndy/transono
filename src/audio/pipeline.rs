use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use anyhow::Result;

use crate::audio::audio_buffer::{CaptureSide, PipelineSide};

pub struct AudioPipeline {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AudioPipeline {
    pub fn new(
        mut capture: CaptureSide,
        mut playback: PipelineSide,
    ) -> Self {
        let running = Arc::new(AtomicBool::new(true));

        let thread_running = Arc::clone(&running);

        let thread = thread::spawn(move || {
            while thread_running.load(Ordering::Acquire) {
                //
                // Здесь позже будет:
                //
                // Capture
                //   ↓
                // Rubato 48 -> 24
                //   ↓
                // OpenAI Realtime
                //   ↓
                // Rubato 24 -> 48
                //   ↓
                // Playback
                //

                std::thread::yield_now();
            }

            drop(capture);
            drop(playback);
        });

        Self {
            running,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }

        Ok(())
    }
}

impl Drop for AudioPipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
