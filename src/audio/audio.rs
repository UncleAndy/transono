use bytes::Bytes;
use cpal::SampleFormat;
use bytemuck;

/// Universal audio container.
#[derive(Debug, Clone)]
pub struct Audio {
    format: AudioFormat,
    data: Bytes,
}

impl Audio {
    pub fn new(
        format: AudioFormat,
        data: Bytes,
    ) -> Self {
        Self {
            format,
            data,
        }
    }

    pub fn view<T: bytemuck::Pod>(
        &self,
    ) -> Result<&[T], bytemuck::PodCastError> {

        Ok(bytemuck::try_cast_slice(
            self.bytes()
        )?)
    }

    pub fn format(&self) -> &AudioFormat {
        &self.format
    }
    pub fn bytes(&self) -> &Bytes {
        &self.data
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
