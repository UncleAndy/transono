use crate::core::error::Result;
use crate::audio::{Audio, PcmAudio};

pub enum Processor {
    Audio(Box<dyn AudioProcessor>),
    Dsp(Box<dyn DspProcessor>),
}

/// Любой обработчик аудиопотока.
pub trait AudioProcessor: Send {
    /// Обрабатывает порцию аудиоданных
    fn process(
        &mut self,
        input: &mut Audio
    )
    -> Result<()>;
}

pub struct IdentityProcessor;

impl AudioProcessor for IdentityProcessor {
    fn process(
        &mut self,
        audio: &mut Audio,
    ) -> Result<()> {
        Ok(())
    }
}

/// Внутренний обработчик в формате внутреннего представления аудо-данных
pub trait DspProcessor: Send {
    /// Обрабатывает порцию аудиоданных
    fn process(
        &mut self,
        input: &mut PcmAudio
    ) -> Result<()>;
}
