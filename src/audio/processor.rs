use crate::core::error::Result;
use crate::audio::{Audio, PcmAudio};
use crate::audio::processors::resampler::Resampler;
use crate::audio::processors::channel_converter::ChannelConverter;
use crate::audio::diagnost::wav_dump::WavDump;

pub enum Processor {
    Identity(IdentityProcessor),
    Resampler(Resampler),
    ChannelConverter(ChannelConverter),
    WavDump(WavDump),
}

impl Processor {
    pub fn process_audio(&mut self, audio: &mut Audio) -> Result<()> {
        match self {
            Self::Identity(p) => p.process(audio),
            _ => panic!("Expected AudioProcessor, got DSP processor"),
        }
    }

    pub fn process_dsp(&mut self, pcm: &mut PcmAudio) -> Result<()> {
        match self {
            Self::Resampler(p) => p.process(pcm),
            Self::ChannelConverter(p) => p.process(pcm),
            Self::WavDump(p) => p.process(pcm),
            Self::Identity(_) => panic!("Expected DSP processor, got Audio processor"),
        }
    }

    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Identity(_))
    }

    pub fn is_dsp(&self) -> bool {
        !self.is_audio()
    }
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
        _audio: &mut Audio,
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
