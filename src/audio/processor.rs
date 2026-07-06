use crate::core::error::Result;
use crate::audio::{Audio, PcmAudio};

/// Любой обработчик аудиопотока.
pub trait AudioProcessor: Send {
    /// Обрабатывает порцию аудиоданных
    fn process(
        &mut self,
        input: Audio
    )
    -> Result<Audio>;
}

pub struct IdentityProcessor;

impl AudioProcessor for IdentityProcessor {
    fn process(
        &mut self,
        audio: Audio,
    ) -> Result<Audio> {
        Ok(audio)
    }
}

/// Внутренний обработчик в формате внутреннего представления аудо-данных
pub trait DspProcessor: Send {
    /// Обрабатывает порцию аудиоданных
    fn process(
        &mut self,
        input: PcmAudio
    )
        -> Result<PcmAudio>;
}
