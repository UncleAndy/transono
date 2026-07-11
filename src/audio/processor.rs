use crate::core::error::Result;
use crate::audio::{Audio, PcmAudio};
use crate::audio::processors::resampler::Resampler;
use crate::audio::processors::channel_converter::ChannelConverter;
use crate::audio::diagnost::wav_dump::WavDump;

pub enum Processor {
    Identity(IdentityProcessor),
    Resampler(Resampler),
    ChannelConverter(ChannelConverter),
    WavDumpDiag(WavDump),
}

impl Processor {
    pub fn process_audio(&mut self, audio: &mut Audio) -> Result<bool> {
        match self {
            Self::Identity(p) => p.process(audio),
            _ => panic!("Expected AudioProcessor, got DSP processor"),
        }
    }

    pub fn process_dsp(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        match self {
            Self::Resampler(p) => p.process(pcm),
            Self::ChannelConverter(p) => p.process(pcm),
            Self::WavDumpDiag(p) => p.process(pcm),
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
    /// Обрабатывает аудиоблок.
    ///
    /// Возвращает:
    /// - `Ok(true)`  — выходные данные готовы;
    /// - `Ok(false)` — требуется больше входных данных;
    /// - `Err(...)`  — ошибка обработки.
    fn process(
        &mut self,
        input: &mut Audio
    )
    -> Result<bool>;
}

pub struct IdentityProcessor;

impl AudioProcessor for IdentityProcessor {
    fn process(
        &mut self,
        _audio: &mut Audio,
    ) -> Result<bool> {
        Ok(true)
    }
}

/// Внутренний обработчик в формате внутреннего представления аудо-данных
pub trait DspProcessor: Send {
    /// Обрабатывает аудиоблок.
    ///
    /// Возвращает:
    /// - `Ok(true)`  — выходные данные готовы;
    /// - `Ok(false)` — требуется больше входных данных;
    /// - `Err(...)`  — ошибка обработки.
    fn process(
        &mut self,
        input: &mut PcmAudio
    ) -> Result<bool>;
}
