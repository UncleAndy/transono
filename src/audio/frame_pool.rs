//! Пул заранее выделенных аудиокадров.

use anyhow::{bail, Result};

use crate::audio::frame::{AudioFrame, FrameId};

pub struct FramePool {
    frames: Box<[AudioFrame]>,
    free: Vec<FrameId>,
}

impl FramePool {
    /// Создает пул фиксированного размера.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        assert!(capacity <= u32::MAX as usize);

        let mut frames = Vec::with_capacity(capacity);

        for _ in 0..capacity {
            frames.push(AudioFrame::default());
        }

        let mut free = Vec::with_capacity(capacity);

        // Стек свободных кадров.
        for id in (0..capacity as FrameId).rev() {
            free.push(id);
        }

        Self {
            frames: frames.into_boxed_slice(),
            free,
        }
    }

    /// Общее количество кадров.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.frames.len()
    }

    /// Количество свободных кадров.
    #[inline]
    pub fn available(&self) -> usize {
        self.free.len()
    }

    /// Получить свободный кадр.
    #[inline]
    pub fn acquire(&mut self) -> Result<FrameId> {
        self.free.pop().ok_or_else(|| anyhow::anyhow!("Frame pool exhausted"))
    }

    /// Вернуть кадр обратно в пул.
    #[inline]
    pub fn release(&mut self, id: FrameId) {
        debug_assert!((id as usize) < self.frames.len());

        self.frames[id as usize].clear();
        self.free.push(id);
    }

    /// Неизменяемый доступ.
    #[inline]
    pub fn get(&self, id: FrameId) -> &AudioFrame {
        &self.frames[id as usize]
    }

    /// Изменяемый доступ.
    #[inline]
    pub fn get_mut(&mut self, id: FrameId) -> &mut AudioFrame {
        &mut self.frames[id as usize]
    }
}
