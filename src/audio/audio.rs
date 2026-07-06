use std::fmt;
use std::fmt::{Debug, Formatter};
use bytes::Bytes;
use cpal::SampleFormat;
use rubato::audioadapter::{Adapter, AdapterMut};
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
        spec: AudioSpec,
        src: &[&[S]],
    ) -> Self
    where
        S: AudioSample,
        AudioBuffer<S>: IntoGenericBuffer,
    {
        let frames = src.first().map_or(0, |c| c.len());

        let mut buffer = AudioBuffer::<S>::new(spec, frames);

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
            pcm.spec.clone(),
            &refs,
        ))
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

    pub fn adapter(
        &mut self,
    ) -> PlanarAdapter<f32> {
        PlanarAdapter::new(&mut self.channels)
    }
    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }
    pub fn channel(&self, index: usize) -> &[f32] {
        &self.channels[index]
    }
    pub fn replace_channel(
        &mut self,
        channel: usize,
        samples: &[f32],
    ) {
        let dst = &mut self.channels[channel];

        dst.clear();
        dst.extend_from_slice(samples);
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
    codec: AudioCodecType,

    container: AudioContainer,

    encoding: BinaryEncoding,

    spec: AudioSpec,

    data: Bytes,
}

impl EncodedAudio {
    pub(crate) fn new(
        container: AudioContainer,
        codec: AudioCodecType,
        encoding: BinaryEncoding,
        spec: AudioSpec,
        data: Bytes
    ) -> EncodedAudio {
        Self {
            codec,
            container,
            encoding,
            spec,
            data,
        }
    }

    pub fn encoding(&self) -> &BinaryEncoding {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioContainer {
    Raw,
    Wav,
    Ogg,
    Mp4,
    Flac,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCodecType {
    Raw,
    Opus,
    Mp3,
    Flac,
    Aac,
    Custom(String),
}


/// Audio encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryEncoding {
    Binary(Endianness),
    Base64(Endianness),
    Custom(Endianness, String),
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

pub struct PlanarAdapter<'a, T> {
    channels: &'a mut [Vec<T>],
}

impl<'a, T> PlanarAdapter<'a, T> {
    pub fn new(
        channels: &'a mut [Vec<T>],
    ) -> Self {
        debug_assert!(
            channels
                .windows(2)
                .all(|w| w[0].len() == w[1].len())
        );

        Self { channels }
    }
}

unsafe impl<'a, T> Adapter<'a, T> for PlanarAdapter<'a, T>
where
    T: Copy
{
    #[inline(always)]
    unsafe fn read_sample_unchecked(
        &self,
        channel: usize,
        frame: usize,
    ) -> T {
        let channel = self.channels.get_unchecked(channel);

        *channel.as_ptr().add(frame)
    }

    fn channels(&self) -> usize {
        self.channels.len()
    }

    fn frames(&self) -> usize {
        self.channels
            .first()
            .map_or(0, Vec::len)
    }
}

unsafe impl<'a, T> AdapterMut<'a, T> for PlanarAdapter<'a, T>
where
    T: Copy + Clone
{
    #[inline(always)]
    unsafe fn write_sample_unchecked(
        &mut self,
        channel: usize,
        frame: usize,
        value: &T,
    ) -> bool {
        let channel = self.channels.get_unchecked_mut(channel);

        *channel.as_mut_ptr().add(frame) = *value;

        false
    }
}
