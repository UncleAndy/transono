use bytes::Bytes;
use symphonia::core::audio::AudioSpec;
use crate::audio::{Endianness};

#[derive(Debug, Clone)]
pub struct EncodedAudio {
    format: EncodedAudioFormat,
    data: Bytes,
}

impl EncodedAudio {
    pub(crate) fn new(
        info: EncodedAudioFormat,
        data: Bytes
    ) -> EncodedAudio {
        Self {
            format: info,
            data,
        }
    }

    pub fn container(&self) -> &AudioContainer {
        &self.format.container
    }
    pub fn codec(&self) -> &AudioCodec {
        &self.format.codec
    }
    pub fn encoding(&self) -> &BinaryEncoding {
        &self.format.encoding
    }
    pub fn spec(&self) -> &AudioSpec {
        &self.format.spec
    }
    pub fn bytes(&self) -> &Bytes {
        &self.data
    }
}

#[derive(Debug, Clone)]
pub struct EncodedAudioFormat {
    container: AudioContainer,
    codec: AudioCodec,
    encoding: BinaryEncoding,
    spec: AudioSpec,
}

impl EncodedAudioFormat {
    pub fn new(
        container: AudioContainer,
        codec: AudioCodec,
        encoding: BinaryEncoding,
        spec: AudioSpec,
    ) -> Self {
        Self {
            container,
            codec,
            encoding,
            spec,
        }
    }

    pub(crate) fn container(&self) -> AudioContainer {
        self.container.clone()
    }
    pub(crate) fn codec(&self) -> AudioCodec {
        self.codec.clone()
    }
    pub(crate) fn encoding(&self) -> BinaryEncoding {
        self.encoding.clone()
    }
    pub(crate) fn spec(&self) -> AudioSpec {
        self.spec.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioContainer {
    Raw,
    Wav,
    Caf,
    Ogg,
    Mp3,
    Mp4,
    Flac,
    Matroska,
    Webm,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCodec {
    Pcm(Endianness),
    Opus,
    Vorbis,
    Aac,
    Flac,
    Alac,
    Ldac,
    Mpeg3,
    Custom(String),
}

/// Audio encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryEncoding {
    Binary,
    Base64,
    Custom(String),
}
