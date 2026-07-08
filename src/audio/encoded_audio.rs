use bytes::Bytes;
use symphonia::core::audio::AudioSpec;

use crate::audio::{Endianness};
use crate::core::error::{CoreError, Result};

#[derive(Debug, Clone)]
pub struct EncodedAudio {
    format: EncodedAudioFormat,
    data: Bytes,
}

impl EncodedAudio {
    pub(crate) fn new(
        format: EncodedAudioFormat,
        data: Bytes
    ) -> EncodedAudio {
        if !matches!(format.codec(), AudioCodec::Pcm(_)) {
            panic!("UnsupportedAudioFormat: {:?}", format);
        }

        Self {
            format,
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
    pub fn as_str(&self) -> Result<&str> {
        match self.encoding() {
            BinaryEncoding::Base64 => {
                std::str::from_utf8(self.data.as_ref())
                    .map_err(|e| CoreError::Other(anyhow::Error::from(e)))
            }

            _ => Err(CoreError::Other(anyhow::anyhow!(
            "EncodedAudio is not text"
        ))),
        }
    }
    pub fn into_string(self) -> Result<String> {
        match self.encoding() {
            BinaryEncoding::Base64 => {
                String::from_utf8(self.data.to_vec())
                    .map_err(|e| CoreError::Other(anyhow::Error::from(e)))
            }
            _ => Err(CoreError::Other(anyhow::anyhow!("EncodedAudio is not text"))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncodedAudioFormat {
    pub(crate) container: AudioContainer,
    pub(crate) codec: AudioCodec,
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
    pub fn spec(&self) -> AudioSpec {
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
    Pcm(PcmFormat),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcmFormat {
    I16(Endianness),
    I24(Endianness),
    I32(Endianness),
    F32(Endianness),
    F64(Endianness),
    U8,
}
impl PcmFormat {
    pub fn sample_size(&self) -> usize {
        match self {
            Self::U8 => 1,
            Self::I16(_) => 2,
            Self::I24(_) => 3,
            Self::I32(_) => 4,
            Self::F32(_) => 4,
            Self::F64(_) => 8,
        }
    }

    pub fn encode_sample(
        &self,
        sample: f32,
        output: &mut [u8],
    ) {
        match self {
            Self::U8 => {
                output[0] = ((sample.clamp(-1.0, 1.0) + 1.0) * 127.5) as u8;
            }

            Self::I16(Endianness::Little) => {
                output[..2].copy_from_slice(
                    &((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .to_le_bytes(),
                );
            }

            Self::I16(Endianness::Big) => {
                output[..2].copy_from_slice(
                    &((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .to_be_bytes(),
                );
            }

            Self::I24(Endianness::Little) => {
                let value =
                    (sample.clamp(-1.0, 1.0) * 8_388_607.0) as i32;

                let bytes = value.to_le_bytes();

                output[0] = bytes[0];
                output[1] = bytes[1];
                output[2] = bytes[2];
            }

            Self::I24(Endianness::Big) => {
                let value =
                    (sample.clamp(-1.0, 1.0) * 8_388_607.0) as i32;

                let bytes = value.to_be_bytes();

                output[0] = bytes[1];
                output[1] = bytes[2];
                output[2] = bytes[3];
            }

            Self::I32(Endianness::Little) => {
                output[..4].copy_from_slice(
                    &((sample.clamp(-1.0, 1.0) * i32::MAX as f32) as i32)
                        .to_le_bytes(),
                );
            }

            Self::I32(Endianness::Big) => {
                output[..4].copy_from_slice(
                    &((sample.clamp(-1.0, 1.0) * i32::MAX as f32) as i32)
                        .to_be_bytes(),
                );
            }

            Self::F32(Endianness::Little) => {
                output[..4].copy_from_slice(
                    &sample.to_le_bytes(),
                );
            }

            Self::F32(Endianness::Big) => {
                output[..4].copy_from_slice(
                    &sample.to_be_bytes(),
                );
            }

            Self::F64(Endianness::Little) => {
                output[..8].copy_from_slice(
                    &(sample as f64).to_le_bytes(),
                );
            }

            Self::F64(Endianness::Big) => {
                output[..8].copy_from_slice(
                    &(sample as f64).to_be_bytes(),
                );
            }
        }
    }

    pub fn decode_sample(
        &self,
        input: &[u8],
    ) -> f32 {
        match self {
            Self::U8 => {
                (input[0] as f32 / 127.5) - 1.0
            }

            Self::I16(Endianness::Little) => {
                i16::from_le_bytes([
                    input[0],
                    input[1],
                ]) as f32 / i16::MAX as f32
            }

            Self::I16(Endianness::Big) => {
                i16::from_be_bytes([
                    input[0],
                    input[1],
                ]) as f32 / i16::MAX as f32
            }

            Self::I24(Endianness::Little) => {
                let value = i32::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    if input[2] & 0x80 != 0 { 0xff } else { 0x00 },
                ]);

                value as f32 / 8_388_607.0
            }

            Self::I24(Endianness::Big) => {
                let value = i32::from_be_bytes([
                    if input[0] & 0x80 != 0 { 0xff } else { 0x00 },
                    input[0],
                    input[1],
                    input[2],
                ]);

                value as f32 / 8_388_607.0
            }

            Self::I32(Endianness::Little) => {
                i32::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ]) as f32 / i32::MAX as f32
            }

            Self::I32(Endianness::Big) => {
                i32::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ]) as f32 / i32::MAX as f32
            }

            Self::F32(Endianness::Little) => {
                f32::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ])
            }

            Self::F32(Endianness::Big) => {
                f32::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ])
            }

            Self::F64(Endianness::Little) => {
                f64::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                    input[4],
                    input[5],
                    input[6],
                    input[7],
                ]) as f32
            }

            Self::F64(Endianness::Big) => {
                f64::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                    input[4],
                    input[5],
                    input[6],
                    input[7],
                ]) as f32
            }
        }
    }
}
