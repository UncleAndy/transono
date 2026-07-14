use std::sync::{Arc, Mutex};
use crate::audio::PcmAudio;
use symphonia::core::audio::AudioSpec;

pub struct PcmPool {
    buffers: Mutex<Vec<PcmAudio>>,
}

impl PcmPool {
    pub fn new() -> Self {
        Self {
            buffers: Mutex::new(Vec::new()),
        }
    }

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

    pub fn release(&self, pcm: PcmAudio) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(pcm);
    }
}

pub type SharedPcmPool = Arc<PcmPool>;
