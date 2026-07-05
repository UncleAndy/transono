use std::fmt;
use std::fmt::{Debug, Formatter};
use bytes::Bytes;
use cpal::SampleFormat;
use symphonia::core::audio::GenericAudioBuffer;

/// Universal audio container.
pub struct Audio {
    format: AudioFormat,
    buffer: GenericAudioBuffer,
}

impl Audio {
    pub fn new(
        format: AudioFormat,
        buffer: GenericAudioBuffer,
    ) -> Self {
        debug_assert_eq!(
            format.channels as usize,
            buffer.spec().channels().count(),
        );
        debug_assert_eq!(
            format.sample_rate,
            buffer.spec().rate(),
        );

        Self {
            format,
            buffer,
        }
    }

    pub fn format(&self) -> &AudioFormat {
        &self.format
    }
    pub fn buffer(&self) -> &GenericAudioBuffer {
        &self.buffer
    }
    pub fn buffer_mut(&mut self) -> &mut GenericAudioBuffer {
        &mut self.buffer
    }
    pub fn replace(
        &mut self,
        format: AudioFormat,
        buffer: GenericAudioBuffer,
    ) {
        debug_assert_eq!(
            format.channels as usize,
            buffer.spec().channels().count(),
        );
        debug_assert_eq!(
            format.sample_rate,
            buffer.spec().rate(),
        );

        self.format = format;
        self.buffer = buffer;
    }
}

impl Debug for Audio {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Audio")
            .field("format", &self.format)
            .field("sample_format", &self.format.sample_format)
            .field("frames", &self.buffer.frames())
            .finish()
    }
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
