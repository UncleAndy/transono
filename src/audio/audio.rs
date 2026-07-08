use std::fmt;
use std::fmt::{Debug, Formatter};
use std::time::Instant;
use symphonia::core::audio::conv::{ConvertibleSample, FromSample};
use symphonia::core::audio::{AudioBuffer, AudioMut, AudioSpec, GenericAudioBuffer};
use symphonia::core::audio::sample::{i24, u24, Sample};

use crate::audio::{PcmAudio, PcmFormat};
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

    pub(crate) fn from_planar<S>(
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
            sequence: 0,
            capture_timestamp: Instant::now(),
            processing_timestamp: Instant::now(),
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

impl_audio_from!(f32, F32);
impl_audio_from!(f64, F64);
impl_audio_from!(u8, U8);
impl_audio_from!(u16, U16);
impl_audio_from!(u24, U24);
impl_audio_from!(u32, U32);
impl_audio_from!(i8, S8);
impl_audio_from!(i16, S16);
impl_audio_from!(i24, S24);
impl_audio_from!(i32, S32);

/// Audio sample layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AudioFormat {
    /// Samples per second.
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u16,
    /// Sample representation.
    pub sample_format: PcmFormat,
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
