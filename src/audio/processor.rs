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

/// A wrapper for various audio and DSP processors.
///
/// Can represent either a high-level [`AudioProcessor`] or a low-level
/// [`DspProcessor`]. Used to build processing pipelines.
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
    /// Processes a high-level [`Audio`] chunk.
    ///
    /// # Arguments
    ///
    /// * `audio` - A mutable reference to the [`Audio`] chunk to be processed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the processing was successful and output is available in the buffer,
    /// `Ok(false)` if more data is needed, or an error.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Processing`] if this variant is a DSP-only processor.
    pub fn process_audio(&mut self, audio: &mut Audio) -> Result<bool> {
        match self {
            Self::Identity(p) => AudioProcessor::process(p, audio),
            Self::Pipeline(p) => AudioProcessor::process(p.as_mut(), audio),
            _ => Err(CoreError::Processing("Expected AudioProcessor, got DSP processor".to_string())),
        }
    }

    /// Processes low-level [`PcmAudio`].
    ///
    /// # Arguments
    ///
    /// * `pcm` - A mutable reference to the [`PcmAudio`] buffer to be processed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the processing was successful and output is available,
    /// `Ok(false)` if more data is needed, or an error.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Processing`] if this variant is an [`AudioProcessor`] only.
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

    /// Returns `true` if this is a high-level [`AudioProcessor`].
    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Identity(_) | Self::Pipeline(_))
    }

    /// Returns `true` if this is a low-level [`DspProcessor`].
    pub fn is_dsp(&self) -> bool {
        match self {
            Self::Pipeline(_) => true,
            _ => !self.is_audio(),
        }
    }
}

/// A high-level audio processor that operates on [`Audio`] chunks.
pub trait AudioProcessor: Send {
    /// Processes an audio chunk.
    ///
    /// # Arguments
    ///
    /// * `input` - A mutable reference to the [`Audio`] chunk.
    ///
    /// # Returns
    ///
    /// Returns:
    /// - `Ok(true)`  — output data is ready;
    /// - `Ok(false)` — needs more input data;
    /// - `Err(...)`  — processing failure.
    fn process(&mut self, input: &mut Audio) -> Result<bool>;
}

/// A processor that does nothing and passes audio through.
pub struct IdentityProcessor;

impl AudioProcessor for IdentityProcessor {
    fn process(&mut self, _audio: &mut Audio) -> Result<bool> {
        Ok(true)
    }
}

/// A low-level DSP processor that operates on [`PcmAudio`].
pub trait DspProcessor: Send {
    /// Processes a PCM audio block.
    ///
    /// # Arguments
    ///
    /// * `input` - A mutable reference to the [`PcmAudio`] block.
    ///
    /// # Returns
    ///
    /// Returns:
    /// - `Ok(true)`  — output data is ready;
    /// - `Ok(false)` — needs more input data;
    /// - `Err(...)`  — processing failure.
    fn process(&mut self, input: &mut PcmAudio) -> Result<bool>;
}

/// A trait for processing chains (Pipelines) that combine multiple processors.
pub trait Pipeline: AudioProcessor + DspProcessor + Send {
    /// Adds a processor to the chain.
    ///
    /// # Arguments
    ///
    /// * `processor` - The [`Processor`] to append to the pipeline.
    fn add(&mut self, processor: Processor);

    /// Clears the processing chain.
    fn clear(&mut self);

    /// Processes an audio block as part of a stream, returning result and duration.
    ///
    /// # Arguments
    ///
    /// * `audio` - The input [`Audio`] block.
    ///
    /// # Returns
    ///
    /// Returns a [`Result`] containing an optional tuple of processed [`Audio`] and the [`Duration`] it took.
    fn process_stream(&mut self, audio: Audio) -> Result<Option<(Audio, Duration)>>;
}
