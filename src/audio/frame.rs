//! Audio frame passed between threads.
//!
//! All frames have a fixed buffer size.
//! The `len` field indicates the number of valid samples.

/// Maximum number of samples in a single frame.
///
/// Must be greater than the maximum CPAL callback size.
/// 2048 samples at 48 kHz correspond to ≈42.7 ms.
pub const FRAME_CAPACITY: usize = 4096;

/// Index of a frame in the pool.
pub type FrameId = u32;

/// A single audio frame.
#[derive(Debug)]
pub struct AudioFrame {
    /// Number of valid samples.
    pub len: usize,

    /// PCM F32 samples.
    pub samples: [f32; FRAME_CAPACITY],
}

impl Default for AudioFrame {
    fn default() -> Self {
        Self {
            len: 0,
            samples: [0.0; FRAME_CAPACITY],
        }
    }
}

impl AudioFrame {
    /// Clears the frame before reuse.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the valid part of the buffer as a slice.
    #[inline(always)]
    pub fn samples(&self) -> &[f32] {
        &self.samples[..self.len]
    }

    /// Returns the valid part of the buffer as a mutable slice.
    #[inline(always)]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples[..self.len]
    }

    /// Copies data from a slice into the frame.
    #[inline(always)]
    pub fn copy_from(&mut self, input: &[f32]) -> bool {
        if input.len() > FRAME_CAPACITY {
            return false;
        }

        self.len = input.len();
        self.samples[..input.len()].copy_from_slice(input);
        true
    }
}
