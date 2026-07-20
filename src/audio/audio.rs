//! Universal audio container and device sample layout.
//!
//! [`Audio`] wraps a shared [`GenericAudioBuffer`](symphonia::core::audio::GenericAudioBuffer)
//! plus a capture timestamp. DSP stages usually convert to [`PcmAudio`] (`f32` planar).

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Instant;

use symphonia::core::audio::conv::{ConvertibleSample, FromSample};
use symphonia::core::audio::{AudioBuffer, AudioMut, AudioSpec, GenericAudioBuffer, Channels};
use symphonia::core::audio::sample::{i24, u24, Sample};

use crate::audio::{PcmAudio, PcmFormat, EncodedAudioFormat, AudioCodec, Endianness};
use crate::core::error::Result;

/// Reference-counted audio chunk with a capture timestamp.
///
/// Cheap to clone (`Arc` over the underlying buffer). Prefer converting to
/// [`PcmAudio`] for DSP rather than mutating sample formats in place.
#[derive(Clone)]
pub struct Audio {
    buffer: Arc<GenericAudioBuffer>,
    capture_timestamp: Instant,
}

impl Audio {
    /// Wrap a buffer and stamp capture time as `Instant::now()`.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A [`GenericAudioBuffer`] containing the audio samples.
    pub fn new(
        buffer: GenericAudioBuffer,
    ) -> Self {
        Self {
            buffer: Arc::new(buffer),
            capture_timestamp: Instant::now(),
        }
    }

    /// Wrap a buffer with an explicit capture timestamp.
    ///
    /// # Arguments
    ///
    /// * `buffer` - A [`GenericAudioBuffer`] containing the audio samples.
    /// * `timestamp` - The precise moment when this audio was captured.
    pub fn new_with_timestamp(
        buffer: GenericAudioBuffer,
        timestamp: Instant,
    ) -> Self {
        Self {
            buffer: Arc::new(buffer),
            capture_timestamp: timestamp,
        }
    }

    /// Instant when this chunk was captured (or assigned).
    pub fn capture_timestamp(&self) -> Instant {
        self.capture_timestamp
    }

    /// Override the capture timestamp.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The new capture timestamp to assign to this chunk.
    pub fn set_capture_timestamp(&mut self, timestamp: Instant) {
        self.capture_timestamp = timestamp;
    }

    /// Duration implied by frame count and sample rate (zero if rate is 0).
    pub fn duration(&self) -> std::time::Duration {
        let frames = self.buffer.frames() as u64;
        let rate = self.buffer.spec().rate() as u64;
        if rate == 0 {
            return std::time::Duration::from_secs(0);
        }
        std::time::Duration::from_nanos(frames * 1_000_000_000 / rate)
    }

    /// Shared reference to the underlying sample buffer.
    pub fn buffer(&self) -> Arc<GenericAudioBuffer> {
        self.buffer.clone()
    }

    /// Copy samples into a planar destination (`dst[channel][frame]`).
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable slice of mutable sample slices. Each inner slice represents one channel.
    ///
    /// # Type Parameters
    ///
    /// * `S` - The target sample type, must implement [`Sample`] and [`ConvertibleSample`].
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

    /// Convert this chunk into a newly allocated [`PcmAudio`] (`f32` planar).
    ///
    /// # Returns
    ///
    /// Returns a [`Result`] containing the new [`PcmAudio`] buffer.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the conversion or memory allocation fails.
    pub fn to_pcm(
        &self,
    ) -> Result<PcmAudio> {
        let buffer = self.buffer();
        let spec = buffer.spec().clone();

        let mut pcm = PcmAudio::new(spec, buffer.frames());
        self.to_pcm_into(&mut pcm)?;

        Ok(pcm)
    }

    /// Convert into an existing [`PcmAudio`], resizing it if needed.
    ///
    /// Prefer this over [`Self::to_pcm`] on hot paths when a buffer is pooled.
    ///
    /// # Arguments
    ///
    /// * `pcm` - A mutable reference to the target [`PcmAudio`] buffer.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the internal symphonia buffer conversion fails.
    pub fn to_pcm_into(
        &self,
        pcm: &mut PcmAudio,
    ) -> Result<()> {
        let buffer = self.buffer();
        let spec = buffer.spec().clone();

        if pcm.spec != spec || pcm.frames() != buffer.frames() {
            pcm.resize(buffer.frames(), spec.channels().count());
            pcm.spec = spec.clone();
        }

        pcm.capture_timestamp = self.capture_timestamp;
        pcm.processing_timestamp = Instant::now();

        let frames = pcm.frames();
        let channel_count = spec.channels().count();

        if frames == 0 || channel_count == 0 {
            return Ok(());
        }

        if channel_count == 1 {
            let mut slices = [&mut pcm.data[..frames]];
            buffer.copy_to_slice_planar(&mut slices);
        } else if channel_count == 2 {
            let (s0, s1) = pcm.data.split_at_mut(frames);
            let mut slices = [s0, &mut s1[..frames]];
            buffer.copy_to_slice_planar(&mut slices);
        } else {
            let mut slices: Vec<&mut [f32]> = pcm.data
                .chunks_exact_mut(frames)
                .take(channel_count)
                .collect();
            buffer.copy_to_slice_planar(&mut slices);
        }

        Ok(())
    }

    /// Creates a new [`Audio`] instance from [`PcmAudio`].
    ///
    /// This performs an allocation as it converts the planar `f32` data back into
    /// a [`GenericAudioBuffer`].
    ///
    /// # Arguments
    ///
    /// * `pcm` - The source [`PcmAudio`] buffer.
    ///
    /// # Returns
    ///
    /// Returns a [`Result`] containing the new [`Audio`] wrapper.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the conversion fails.
    pub fn from_pcm(
        pcm: &PcmAudio,
    ) -> Result<Self> {
        let frames = pcm.frames();
        let channel_count = pcm.channel_count();

        if frames == 0 || channel_count == 0 {
            return Ok(Self::new(GenericAudioBuffer::F32(AudioBuffer::new(pcm.spec.clone(), 0))));
        }

        let mut audio = if channel_count == 1 {
            let slices = [&pcm.data[..frames]];
            Self::from_planar::<f32>(pcm.spec.clone(), &slices)
        } else if channel_count == 2 {
            let (s0, s1) = pcm.data.split_at(frames);
            let slices = [s0, &s1[..frames]];
            Self::from_planar::<f32>(pcm.spec.clone(), &slices)
        } else {
            let refs: Vec<&[f32]> = pcm.data
                .chunks_exact(frames)
                .take(channel_count)
                .collect();
            Self::from_planar::<f32>(pcm.spec.clone(), &refs)
        };

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

impl AudioFormat {
    /// Returns the symphonia [`AudioSpec`] for this format.
    pub fn spec(&self) -> AudioSpec {
        AudioSpec::new(self.sample_rate, Channels::Discrete(self.channels))
    }

    /// Returns the size of a single audio frame in bytes.
    pub fn frame_size(&self) -> usize {
        self.channels as usize * self.sample_format.sample_size()
    }
}

impl From<EncodedAudioFormat> for AudioFormat {
    fn from(format: EncodedAudioFormat) -> Self {
        let spec = format.spec();
        Self {
            sample_rate: spec.rate(),
            channels: spec.channels().count() as u16,
            sample_format: match format.codec() {
                AudioCodec::Pcm(pcm) => pcm,
                _ => PcmFormat::F32(Endianness::Little),
            },
        }
    }
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
