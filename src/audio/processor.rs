use crate::audio::diagnost::indicator::Indicator;
use crate::audio::diagnost::wav_dump::WavDump;
use crate::audio::processors::channel_converter::ChannelConverter;
use crate::audio::processors::denoiser::Denoiser;
use crate::audio::processors::resampler::Resampler;
use crate::audio::processors::compressor::Compressor;
use crate::audio::processors::normalizer::Normalizer;
use crate::audio::{Audio, PcmAudio, AudioPipeline};
use crate::core::error::Result;

pub enum Processor {
    Identity(IdentityProcessor),
    Denoiser(Denoiser),
    Resampler(Resampler),
    ChannelConverter(ChannelConverter),
    Compressor(Compressor),
    Normalizer(Normalizer),
    Pipeline(Box<AudioPipeline>),
    IndicatorDiag(Indicator),
    WavDumpDiag(WavDump),
}

impl Processor {
    pub fn process_audio(&mut self, audio: &mut Audio) -> Result<bool> {
        match self {
            Self::Identity(p) => p.process(audio),
            Self::Pipeline(p) => p.process_audio(audio),
            _ => panic!("Expected AudioProcessor, got DSP processor"),
        }
    }

    pub fn process_dsp(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        match self {
            Self::Denoiser(p) => p.process(pcm),
            Self::Resampler(p) => p.process(pcm),
            Self::ChannelConverter(p) => p.process(pcm),
            Self::Compressor(p) => p.process(pcm),
            Self::Normalizer(p) => p.process(pcm),
            Self::Pipeline(p) => p.process_dsp(pcm),
            Self::IndicatorDiag(p) => p.process(pcm),
            Self::WavDumpDiag(p) => p.process(pcm),
            Self::Identity(_) => panic!("Expected DSP processor, got Audio processor"),
        }
    }

    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Identity(_) | Self::Pipeline(_))
    }

    pub fn is_dsp(&self) -> bool {
        match self {
            Self::Pipeline(_) => true,
            _ => !self.is_audio(),
        }
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
    fn process(&mut self, input: &mut Audio) -> Result<bool>;
}

pub struct IdentityProcessor;

impl AudioProcessor for IdentityProcessor {
    fn process(&mut self, _audio: &mut Audio) -> Result<bool> {
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
    fn process(&mut self, input: &mut PcmAudio) -> Result<bool>;
}

/// Трейт для цепочки обработки (Pipeline), объединяющий оба типа процессоров.
pub trait Pipeline: AudioProcessor + DspProcessor {}
