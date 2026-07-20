use std::sync::{Arc, Mutex};
use crate::audio::PcmAudio;
use symphonia::core::audio::AudioSpec;

/// A pool for reusing `PcmAudio` buffers to avoid frequent allocations.
pub struct PcmPool {
    buffers: Mutex<Vec<PcmAudio>>,
}

impl PcmPool {
    /// Creates a new empty PCM pool.
    pub fn new() -> Self {
        Self {
            buffers: Mutex::new(Vec::new()),
        }
    }

    /// Acquires a `PcmAudio` buffer from the pool or creates a new one if the pool is empty.
    pub fn acquire(&self, spec: AudioSpec, frames: usize) -> PcmAudio {
        let mut buffers = self.buffers.lock().unwrap();
        if let Some(mut pcm) = buffers.pop() {
            pcm.spec = spec.clone();
            pcm.resize(frames, spec.channels().count());
            pcm
        } else {
            PcmAudio::new(spec, frames)
        }
    }

    /// Releases a `PcmAudio` buffer back into the pool for future reuse.
    pub fn release(&self, pcm: PcmAudio) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(pcm);
    }
}

/// A thread-safe shared reference to a `PcmPool`.
pub type SharedPcmPool = Arc<PcmPool>;
