use bytes::Bytes;
use cpal::SampleFormat;

/// Universal audio container.
#[derive(Debug, Clone)]
pub struct Audio {
    pub format: AudioFormat,
    pub data: Bytes,
}

pub struct EncodedAudio {
    pub encoding: AudioEncoding,
    pub data: Bytes,
}

/// Byte order for PCM encoded audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

/// Audio sample layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
