use std::time::Duration;
use crate::audio::diagnost::indicator::Indicator;
use crate::audio::diagnost::wav_dump::WavDump;
use crate::audio::processors::channel_converter::ChannelConverter;
use crate::audio::processors::denoiser::Denoiser;
use crate::audio::processors::resampler::Resampler;
use crate::audio::processors::compressor::Compressor;
use crate::audio::processors::normalizer::Normalizer;
use crate::audio::{Audio, PcmAudio};
use crate::core::error::{CoreError, Result};

pub enum Processor {
    Identity(IdentityProcessor),
    Denoiser(Denoiser),
    Resampler(Resampler),
    ChannelConverter(ChannelConverter),
    Compressor(Compressor),
    Normalizer(Normalizer),
    Pipeline(Box<dyn Pipeline>),
    IndicatorDiag(Indicator),
    WavDumpDiag(WavDump),
}

impl Processor {
    pub fn process_audio(&mut self, audio: &mut Audio) -> Result<bool> {
        match self {
            Self::Identity(p) => AudioProcessor::process(p, audio),
            Self::Pipeline(p) => AudioProcessor::process(p.as_mut(), audio),
            _ => Err(CoreError::Processing("Expected AudioProcessor, got DSP processor".to_string())),
        }
    }

    pub fn process_dsp(&mut self, pcm: &mut PcmAudio) -> Result<bool> {
        match self {
            Self::Denoiser(p) => DspProcessor::process(p, pcm),
            Self::Resampler(p) => DspProcessor::process(p, pcm),
            Self::ChannelConverter(p) => DspProcessor::process(p, pcm),
            Self::Compressor(p) => DspProcessor::process(p, pcm),
            Self::Normalizer(p) => DspProcessor::process(p, pcm),
            Self::Pipeline(p) => DspProcessor::process(p.as_mut(), pcm),
            Self::IndicatorDiag(p) => DspProcessor::process(p, pcm),
            Self::WavDumpDiag(p) => DspProcessor::process(p, pcm),
            Self::Identity(_) => Err(CoreError::Processing("Expected DSP processor, got Audio processor".to_string())),
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
pub trait Pipeline: AudioProcessor + DspProcessor + Send {
    /// Добавляет процессор в цепочку.
    fn add(&mut self, processor: Processor);

    /// Очищает цепочку процессоров.
    fn clear(&mut self);

    /// Обрабатывает блок аудио как часть потока, возвращая результат и время обработки.
    fn process_stream(&mut self, audio: Audio) -> Result<Option<(Audio, Duration)>>;
}
