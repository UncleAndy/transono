use std::fmt;
use std::fmt::{Debug, Formatter};
use bytes::Bytes;
use cpal::SampleFormat;
use symphonia::core::audio::{AudioBuffer, GenericAudioBuffer};
use symphonia::core::audio::sample::{i24, u24};

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

    pub fn from_f32(buffer: AudioBuffer<f32>) -> Self {
        Self {
            buffer: GenericAudioBuffer::F32(buffer),
        }
    }

    pub fn from_i16(buffer: AudioBuffer<i16>) -> Self {
        Self {
            buffer: GenericAudioBuffer::S16(buffer),
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
}

impl Debug for Audio {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Audio")
            .field("frames", &self.buffer.frames())
            .finish()
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
