//! Предварительно выделенный пул аудиокадров.
//!
//! FramePool не занимается управлением жизненным циклом кадров.
//! Он только предоставляет быстрый доступ к памяти.
//!
//! Владение FrameId определяется исключительно очередями rtrb.

use std::cell::UnsafeCell;

use crate::audio::frame::{AudioFrame, FrameId};

pub struct FramePool {
    frames: Box<[UnsafeCell<AudioFrame>]>,
}

// SAFETY:
//
// Каждый AudioFrame одновременно принадлежит только одному владельцу.
//
// Жизненный цикл:
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
// Один и тот же FrameId никогда одновременно не находится
// в двух очередях.
//
// Поэтому одновременно существовать двух &mut AudioFrame
// для одного кадра не может.
//
unsafe impl Sync for FramePool {}

impl FramePool {
    /// Создает пул фиксированного размера.
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

    /// Количество кадров.
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.frames.len()
    }

    /// Неизменяемый доступ.
    #[inline(always)]
    pub fn get(&self, id: FrameId) -> &AudioFrame {
        debug_assert!((id as usize) < self.frames.len());

        unsafe { &*self.frames[id as usize].get() }
    }

    /// Изменяемый доступ.
    #[inline(always)]
    pub fn get_mut(&self, id: FrameId) -> &mut AudioFrame {
        debug_assert!((id as usize) < self.frames.len());

        unsafe { &mut *self.frames[id as usize].get() }
    }
}
