//! Pre-allocated audio frame pool.
//!
//! FramePool does not manage frame lifetimes.
//! It only provides fast memory access.
//!
//! Ownership of FrameId is determined solely by rtrb queues.

use std::cell::UnsafeCell;

use crate::audio::frame::{AudioFrame, FrameId};

/// A pool of audio frames.
pub struct FramePool {
    frames: Box<[UnsafeCell<AudioFrame>]>,
}

// SAFETY:
//
// Each AudioFrame belongs to only one owner at a time.
//
// Lifecycle:
//
// FreeQueue
//      ↓
// Capture
//      ↓
// FilledQueue
//      ↓
// Pipeline
//      ↓
// FreeQueue
//
// The same FrameId is never in two queues simultaneously.
//
// Therefore, two &mut AudioFrame for the same frame cannot exist at the same time.
//
unsafe impl Sync for FramePool {}

impl FramePool {
    /// Creates a fixed-size pool.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);

        let mut frames = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            frames.push(UnsafeCell::new(AudioFrame::default()));
        }

        Self {
            frames: frames.into_boxed_slice(),
        }
    }

    /// Returns the number of frames in the pool.
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.frames.len()
    }

    /// Immutable access to a frame by ID.
    #[inline(always)]
    pub fn get(&self, id: FrameId) -> &AudioFrame {
        debug_assert!((id as usize) < self.frames.len());

        unsafe { &*self.frames[id as usize].get() }
    }

    /// Mutable access to a frame by ID.
    #[inline(always)]
    pub fn get_mut(&self, id: FrameId) -> &mut AudioFrame {
        debug_assert!((id as usize) < self.frames.len());

        unsafe { &mut *self.frames[id as usize].get() }
    }
}
