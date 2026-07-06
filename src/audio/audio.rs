use std::fmt;
use std::fmt::{Debug, Formatter};
use bytes::Bytes;
use cpal::SampleFormat;

use symphonia::core::audio::conv::{ConvertibleSample, FromSample};
use symphonia::core::audio::{AudioBuffer, AudioMut, AudioSpec, GenericAudioBuffer};
use symphonia::core::audio::sample::{i24, u24, Sample};

use crate::core::error::Result;

/// Universal audio container.
pub struct Audio {
    buffer: GenericAudioBuffer,
}

impl Audio {
    pub fn new(
        buffer: GenericAudioBuffer,
    ) -> Self {
        Self {
            buffer,
        }
    }

    pub fn buffer(&self) -> &GenericAudioBuffer {
        &self.buffer
    }
    pub fn buffer_mut(&mut self) -> &mut GenericAudioBuffer {
        &mut self.buffer
    }
    pub fn replace(
        &mut self,
        buffer: GenericAudioBuffer,
    ) {
        self.buffer = buffer;
    }

    pub fn copy_to_planar<S>(
        &self,
        dst: &mut [&mut [S]],
    )
    where
        S: Sample + ConvertibleSample,
    {
        self.buffer.copy_to_slice_planar(dst);
    }

    pub fn from_planar<S>(
        spec: &AudioSpec,
        src: &[&[S]],
    ) -> Self
    where
        S: AudioSample,
        AudioBuffer<S>: IntoGenericBuffer,
    {
        let frames = src.first().map_or(0, |c| c.len());

        let mut buffer = AudioBuffer::<S>::new(
            spec.clone(),
            frames,
        );

        buffer.render_uninit(Some(frames));

        buffer.copy_from_slice_planar(src);

        Self::new(buffer.into_generic_buffer())
    }

    pub fn to_pcm(
        &self,
    ) -> Result<PcmAudio> {
        let buffer = self.buffer();

        let spec = buffer.spec().clone();

        let mut channels = vec![
            vec![0.0f32; buffer.frames()];
            spec.channels().count()
        ];

        let mut slices: Vec<&mut [f32]> = channels
            .iter_mut()
            .map(Vec::as_mut_slice)
            .collect();

        buffer.copy_to_slice_planar(&mut slices);

        Ok(PcmAudio {
            spec,
            channels,
        })
    }

    pub fn from_pcm(
        pcm: &PcmAudio,
    ) -> Result<Self> {

        let refs: Vec<&[f32]> = pcm
            .channels
            .iter()
            .map(Vec::as_slice)
            .collect();

        Ok(Self::from_planar::<f32>(
            &pcm.spec,
            &refs,
        ))
    }

    pub fn into_buffer(self) -> GenericAudioBuffer {
        self.buffer
    }
}

impl Debug for Audio {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Audio")
            .field("frames", &self.buffer.frames())
            .finish()
    }
}

/// Internal DSP representation.
///
/// The library supports arbitrary sample formats through `Audio`,
/// but all built-in DSP processors currently operate on `f32`.
pub(crate) struct PcmAudio {
    pub spec: AudioSpec,
    pub channels: Vec<Vec<f32>>, // Один Vec на канал.
}

impl PcmAudio {
    pub fn frames(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }
}

#[allow(unused)]
macro_rules! impl_audio_from {
    ($sample:ty, $variant:ident) => {
        impl From<AudioBuffer<$sample>> for Audio {
            fn from(buffer: AudioBuffer<$sample>) -> Self {
                Self {
                    buffer: GenericAudioBuffer::$variant(buffer),
                }
            }
        }
    };
}

#[derive(Debug, Clone)]
pub struct EncodedAudio {
    encoding: AudioEncoding,
    data: Bytes,
}

impl EncodedAudio {
    pub(crate) fn new(encoding: AudioEncoding, data: Bytes) -> EncodedAudio {
        Self {
            encoding,
            data,
        }
    }

    pub fn encoding(&self) -> &AudioEncoding {
        &self.encoding
    }
    pub fn bytes(&self) -> &Bytes {
        &self.data
    }
}

/// Byte order for PCM encoded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

/// Audio sample layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    /// Samples per second.
    pub sample_rate: u32,

    /// Number of channels.
    pub channels: u16,

    /// Sample representation.
    pub sample_format: SampleFormat,
}

/// Audio encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioEncoding {
    /// Raw PCM stream.
    Pcm {
        endianness: Endianness,
    },

    /// Opus encoded audio.
    Opus,

    /// MP3 encoded audio.
    Mp3,

    /// FLAC encoded audio.
    Flac,

    /// AAC encoded audio.
    Aac,

    /// Unknown/custom encoding.
    Custom(String),
}

impl AudioFormat {
    pub const OPENAI_REALTIME: Self = Self {
        sample_rate: 24_000,
        channels: 1,
        sample_format: SampleFormat::I16,
    };
}

pub(crate) trait IntoGenericBuffer {
    fn into_generic_buffer(self) -> GenericAudioBuffer;
}

macro_rules! impl_into_generic {
    ($ty:ty, $variant:ident) => {
        impl IntoGenericBuffer for AudioBuffer<$ty> {
            fn into_generic_buffer(self) -> GenericAudioBuffer {
                GenericAudioBuffer::$variant(self)
            }
        }
    };
}

impl_into_generic!(f32, F32);
impl_into_generic!(f64, F64);
impl_into_generic!(u8, U8);
impl_into_generic!(u16, U16);
impl_into_generic!(u24, U24);
impl_into_generic!(u32, U32);
impl_into_generic!(i8, S8);
impl_into_generic!(i16, S16);
impl_into_generic!(i24, S24);
impl_into_generic!(i32, S32);

pub(crate) trait AudioSample:
Sample
+ FromSample<Self>
+ Send
+ 'static
{}

impl<T> AudioSample for T
where
    T: Sample
    + FromSample<T>
    + Send
    + 'static,
{}
