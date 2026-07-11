use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Instant;

use symphonia::core::audio::conv::{ConvertibleSample, FromSample};
use symphonia::core::audio::{AudioBuffer, AudioMut, AudioSpec, GenericAudioBuffer};
use symphonia::core::audio::sample::{i24, u24, Sample};

use crate::audio::{PcmAudio, PcmFormat};
use crate::core::error::Result;

/// Universal audio container.
#[derive(Clone)]
pub struct Audio {
    buffer: Arc<GenericAudioBuffer>,
    capture_timestamp: Instant,
}

impl Audio {
    pub fn new(
        buffer: GenericAudioBuffer,
    ) -> Self {
        Self {
            buffer: Arc::new(buffer),
            capture_timestamp: Instant::now(),
        }
    }

    pub fn new_with_timestamp(
        buffer: GenericAudioBuffer,
        timestamp: Instant,
    ) -> Self {
        Self {
            buffer: Arc::new(buffer),
            capture_timestamp: timestamp,
        }
    }

    pub fn capture_timestamp(&self) -> Instant {
        self.capture_timestamp
    }

    pub fn duration(&self) -> std::time::Duration {
        let frames = self.buffer.frames() as u64;
        let rate = self.buffer.spec().rate() as u64;
        std::time::Duration::from_nanos(frames * 1_000_000_000 / rate)
    }

    pub fn buffer(&self) -> Arc<GenericAudioBuffer> {
        self.buffer.clone()
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

        let mut pcm = PcmAudio::new(spec, buffer.frames());
        self.to_pcm_into(&mut pcm)?;

        Ok(pcm)
    }

    pub fn to_pcm_into(
        &self,
        pcm: &mut PcmAudio,
    ) -> Result<()> {
        let buffer = self.buffer();
        let spec = buffer.spec().clone();

        if pcm.spec != spec || pcm.frames() != buffer.frames() {
            *pcm = PcmAudio::new(spec, buffer.frames());
        }

        pcm.capture_timestamp = self.capture_timestamp;
        pcm.processing_timestamp = Instant::now();

        let frames = pcm.frames();
        let mut slices: Vec<&mut [f32]> = pcm.data
            .chunks_exact_mut(frames)
            .collect();

        buffer.copy_to_slice_planar(&mut slices);

        Ok(())
    }

    pub fn from_pcm(
        pcm: &PcmAudio,
    ) -> Result<Self> {
        let frames = pcm.frames();
        let refs: Vec<&[f32]> = pcm.data
            .chunks_exact(frames)
            .collect();

        let mut audio = Self::from_planar::<f32>(
            pcm.spec.clone(),
            &refs,
        );

        audio.capture_timestamp = pcm.capture_timestamp;

        Ok(audio)
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
                    buffer: Arc::new(GenericAudioBuffer::$variant(buffer)),
                    capture_timestamp: Instant::now(),
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
