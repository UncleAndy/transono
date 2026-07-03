//! Аудиокадр, передаваемый между потоками.
//!
//! Все кадры имеют фиксированный размер буфера.
//! Поле `len` указывает количество валидных сэмплов.

/// Максимальное количество сэмплов в одном кадре.
///
/// Должно быть больше максимального размера callback CPAL.
/// 2048 сэмплов при 48 кГц соответствуют ≈42.7 мс.
pub const FRAME_CAPACITY: usize = 2048;

/// Индекс кадра в пуле.
pub type FrameId = u32;

/// Один аудиокадр.
#[derive(Debug)]
pub struct AudioFrame {
    /// Количество валидных сэмплов.
    pub len: usize,

    /// PCM F32.
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
    /// Очищает кадр перед повторным использованием.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Возвращает заполненную часть буфера.
    #[inline(always)]
    pub fn samples(&self) -> &[f32] {
        &self.samples[..self.len]
    }

    /// Возвращает заполненную часть буфера для записи.
    #[inline(always)]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples[..self.len]
    }
}
