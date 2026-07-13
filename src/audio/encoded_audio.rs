use audio_samples::ConvertTo;
use i24::{I24, U24};
use bytes::Bytes;
use symphonia::core::audio::{AudioSpec, Channels, Position};
use symphonia::core::audio::Channels::Positioned;
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
                let bytes = self.data.to_vec();
                String::from_utf8(bytes)
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
    pub fn internal_for_voice() -> Self {
        Self {
            container: AudioContainer::Raw,
            codec: AudioCodec::Pcm(PcmFormat::F32(Endianness::Little)),
            encoding: BinaryEncoding::Binary,
            spec: AudioSpec::new(
                48_000,
                Positioned(Position::FRONT_CENTER),
            ),
        }
    }

    pub fn container(&self) -> AudioContainer {
        self.container.clone()
    }
    pub fn codec(&self) -> AudioCodec {
        self.codec.clone()
    }
    pub fn encoding(&self) -> BinaryEncoding {
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

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub enum PcmFormat {
    I8,
    I16(Endianness),
    I24(Endianness),
    I32(Endianness),
    I64(Endianness),
    U8,
    U16(Endianness),
    U24(Endianness),
    U32(Endianness),
    U64(Endianness),
    F32(Endianness),
    F64(Endianness),
    DsdU8,
    DsdU16(Endianness),
    DsdU32(Endianness),
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
            Self::I8 => 1,
            Self::I64(_) => 8,
            Self::U16(_) => 2,
            Self::U24(_) => 3,
            Self::U32(_) => 4,
            Self::U64(_) => 8,
            Self::DsdU8 => 1,
            Self::DsdU16(_) => 2,
            Self::DsdU32(_) => 4,
        }
    }

    // audio_samples преобразует f32 <-> i32.
    // Для хранения в I24 необходимо уменьшить разрядность до 24 бит,
    // а при чтении восстановить её обратно.

    #[inline]
    pub fn encode_sample(&self, sample: f32, out: &mut [u8]) {
        match self {
            //
            // Signed integers
            //
            Self::I8 => {
                out[0] = ((sample.clamp(-1.0, 1.0) * 127.0).round() as i8) as u8;
            }

            Self::I16(Endianness::Little) => {
                let value: i16 = sample.convert_to();
                out[..2].copy_from_slice(&value.to_le_bytes());
            }

            Self::I16(Endianness::Big) => {
                let value: i16 = sample.convert_to();
                out[..2].copy_from_slice(&value.to_be_bytes());
            }

            Self::I24(endian) => {
                let value: i32 = sample.convert_to();
                let value = value >> 8;
                let value = I24::wrapping_from_i32(value);
                write_i24(value, out, *endian);
            }

            Self::I32(Endianness::Little) => {
                let value: i32 = sample.convert_to();
                out[..4].copy_from_slice(&value.to_le_bytes());
            }

            Self::I32(Endianness::Big) => {
                let value: i32 = sample.convert_to();
                out[..4].copy_from_slice(&value.to_be_bytes());
            }

            Self::I64(Endianness::Little) => {
                let value =
                    (sample.clamp(-1.0, 1.0) * i64::MAX as f32)
                        .round() as i64;

                out[..8].copy_from_slice(&value.to_le_bytes());
            }

            Self::I64(Endianness::Big) => {
                let value =
                    (sample.clamp(-1.0, 1.0) * i64::MAX as f32)
                        .round() as i64;

                out[..8].copy_from_slice(&value.to_be_bytes());
            }

            //
            // Unsigned integers
            //
            Self::U8 => {
                let value: u8 = sample.convert_to();
                out[0] = value;
            }

            Self::U16(Endianness::Little) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5)
                        * u16::MAX as f32)
                        .round() as u16;

                out[..2].copy_from_slice(&value.to_le_bytes());
            }

            Self::U16(Endianness::Big) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5)
                        * u16::MAX as f32)
                        .round() as u16;

                out[..2].copy_from_slice(&value.to_be_bytes());
            }

            Self::U24(endian) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5)
                        * ((1u32 << 24) - 1) as f32)
                        .round() as u32;

                write_u24(
                    U24::wrapping_from_u32(value),
                    out,
                    *endian,
                );
            }

            Self::U32(Endianness::Little) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5)
                        * u32::MAX as f32)
                        .round() as u32;

                out[..4].copy_from_slice(&value.to_le_bytes());
            }

            Self::U32(Endianness::Big) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) + 1.0) * 0.5)
                        * u32::MAX as f32)
                        .round() as u32;

                out[..4].copy_from_slice(&value.to_be_bytes());
            }

            Self::U64(Endianness::Little) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) as f64 + 1.0) * 0.5)
                        * u64::MAX as f64)
                        .round() as u64;

                out[..8].copy_from_slice(&value.to_le_bytes());
            }

            Self::U64(Endianness::Big) => {
                let value =
                    (((sample.clamp(-1.0, 1.0) as f64 + 1.0) * 0.5)
                        * u64::MAX as f64)
                        .round() as u64;

                out[..8].copy_from_slice(&value.to_be_bytes());
            }

            //
            // Floating point
            //
            Self::F32(Endianness::Little) => {
                out[..4].copy_from_slice(&sample.to_le_bytes());
            }

            Self::F32(Endianness::Big) => {
                out[..4].copy_from_slice(&sample.to_be_bytes());
            }

            Self::F64(Endianness::Little) => {
                out[..8].copy_from_slice(&(sample as f64).to_le_bytes());
            }

            Self::F64(Endianness::Big) => {
                out[..8].copy_from_slice(&(sample as f64).to_be_bytes());
            }

            //
            // DSD
            //
            Self::DsdU8
            | Self::DsdU16(_)
            | Self::DsdU32(_) => {
                unimplemented!("DSD PCM conversion is not supported");
            }
        }
    }

    pub fn decode_sample(
        &self,
        input: &[u8],
    ) -> f32 {
        match self {
            //
            // Signed integers
            //
            Self::I8 => {
                (input[0] as i8) as f32 / 127.0
            }

            Self::I16(Endianness::Little) => {
                let value = i16::from_le_bytes([
                    input[0],
                    input[1],
                ]);

                value.convert_to()
            }

            Self::I16(Endianness::Big) => {
                let value = i16::from_be_bytes([
                    input[0],
                    input[1],
                ]);

                value.convert_to()
            }

            Self::I24(endian) => {
                let value = read_i24(input, *endian);
                let value = value.to_i32() << 8;
                value.convert_to()
            }

            Self::I32(Endianness::Little) => {
                let value = i32::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ]);

                value.convert_to()
            }

            Self::I32(Endianness::Big) => {
                let value = i32::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ]);

                value.convert_to()
            }

            Self::I64(Endianness::Little) => {
                let value = i64::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                    input[4],
                    input[5],
                    input[6],
                    input[7],
                ]);

                (value as f64 / i64::MAX as f64) as f32
            }

            Self::I64(Endianness::Big) => {
                let value = i64::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                    input[4],
                    input[5],
                    input[6],
                    input[7],
                ]);

                (value as f64 / i64::MAX as f64) as f32
            }

            //
            // Unsigned integers
            //
            Self::U8 => {
                input[0].convert_to()
            }

            Self::U16(Endianness::Little) => {
                let value = u16::from_le_bytes([
                    input[0],
                    input[1],
                ]);

                value as f32 / u16::MAX as f32 * 2.0 - 1.0
            }

            Self::U16(Endianness::Big) => {
                let value = u16::from_be_bytes([
                    input[0],
                    input[1],
                ]);

                value as f32 / u16::MAX as f32 * 2.0 - 1.0
            }

            Self::U24(endian) => {
                let value = read_u24(input, *endian);

                value.to_u32() as f32
                    / ((1u32 << 24) - 1) as f32
                    * 2.0
                    - 1.0
            }

            Self::U32(Endianness::Little) => {
                let value = u32::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ]);

                value as f32 / u32::MAX as f32 * 2.0 - 1.0
            }

            Self::U32(Endianness::Big) => {
                let value = u32::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                ]);

                value as f32 / u32::MAX as f32 * 2.0 - 1.0
            }

            Self::U64(Endianness::Little) => {
                let value = u64::from_le_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                    input[4],
                    input[5],
                    input[6],
                    input[7],
                ]);

                (value as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32
            }

            Self::U64(Endianness::Big) => {
                let value = u64::from_be_bytes([
                    input[0],
                    input[1],
                    input[2],
                    input[3],
                    input[4],
                    input[5],
                    input[6],
                    input[7],
                ]);

                (value as f64 / u64::MAX as f64 * 2.0 - 1.0) as f32
            }

            //
            // Floating point
            //
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

            //
            // DSD
            //
            Self::DsdU8
            | Self::DsdU16(_)
            | Self::DsdU32(_) => {
                unimplemented!("DSD PCM conversion is not supported");
            }
        }
    }
}


#[inline]
fn read_i24(input: &[u8], endian: Endianness) -> I24 {
    match endian {
        Endianness::Little => {
            I24::from_le_bytes([
                input[0],
                input[1],
                input[2],
            ])
        }
        Endianness::Big => {
            I24::from_be_bytes([
                input[0],
                input[1],
                input[2],
            ])
        }
    }
}

#[inline]
fn write_i24(
    value: I24,
    output: &mut [u8],
    endian: Endianness,
) {
    let bytes = match endian {
        Endianness::Little => value.to_le_bytes(),
        Endianness::Big => value.to_be_bytes(),
    };

    output[..3].copy_from_slice(&bytes);
}

#[inline]
fn read_u24(
    input: &[u8],
    endian: Endianness,
) -> U24 {
    match endian {
        Endianness::Little => {
            U24::from_le_bytes([
                input[0],
                input[1],
                input[2],
            ])
        }
        Endianness::Big => {
            U24::from_be_bytes([
                input[0],
                input[1],
                input[2],
            ])
        }
    }
}

#[inline]
fn write_u24(
    value: U24,
    output: &mut [u8],
    endian: Endianness,
) {
    let bytes = match endian {
        Endianness::Little => value.to_le_bytes(),
        Endianness::Big => value.to_be_bytes(),
    };

    output[..3].copy_from_slice(&bytes);
}

#[cfg(test)]
const TEST_SAMPLES: &[f32] = &[
    -1.0,
    -0.75,
    -0.5,
    -0.25,
    0.0,
    0.25,
    0.5,
    0.75,
    1.0,
];

#[cfg(test)]
fn roundtrip(format: PcmFormat) {
    let mut buf = [0u8; 8];

    for &sample in TEST_SAMPLES {
        format.encode_sample(sample, &mut buf);

        let decoded =
            format.decode_sample(&buf);

        let error =
            (decoded - sample).abs();

        assert!(
            error < 0.004,
            "{:?}: {} -> {} (error={})",
            format,
            sample,
            decoded,
            error,
        );
    }
}

#[test]
fn pcm_i8() {
    roundtrip(PcmFormat::I8);
}

#[test]
fn pcm_u8() {
    roundtrip(PcmFormat::U8);
}

#[test]
fn pcm_i16_le() {
    roundtrip(PcmFormat::I16(Endianness::Little));
}

#[test]
fn pcm_i16_be() {
    roundtrip(PcmFormat::I16(Endianness::Big));
}

#[test]
fn pcm_i24_le() {
    roundtrip(PcmFormat::I24(Endianness::Little));
}

#[test]
fn pcm_i24_be() {
    roundtrip(PcmFormat::I24(Endianness::Big));
}

#[test]
fn pcm_u24_le() {
    roundtrip(PcmFormat::U24(Endianness::Little));
}

#[test]
fn pcm_u24_be() {
    roundtrip(PcmFormat::U24(Endianness::Big));
}

#[test]
fn pcm_i32_le() {
    roundtrip(PcmFormat::I32(Endianness::Little));
}

#[test]
fn pcm_i32_be() {
    roundtrip(PcmFormat::I32(Endianness::Big));
}

#[test]
fn pcm_u32_le() {
    roundtrip(PcmFormat::U32(Endianness::Little));
}

#[test]
fn pcm_u32_be() {
    roundtrip(PcmFormat::U32(Endianness::Big));
}

#[test]
fn pcm_i64_le() {
    roundtrip(PcmFormat::I64(Endianness::Little));
}

#[test]
fn pcm_i64_be() {
    roundtrip(PcmFormat::I64(Endianness::Big));
}

#[test]
fn pcm_u64_le() {
    roundtrip(PcmFormat::U64(Endianness::Little));
}

#[test]
fn pcm_u64_be() {
    roundtrip(PcmFormat::U64(Endianness::Big));
}

#[test]
fn pcm_f32_le() {
    roundtrip(PcmFormat::F32(Endianness::Little));
}

#[test]
fn pcm_f32_be() {
    roundtrip(PcmFormat::F32(Endianness::Big));
}

#[test]
fn pcm_f64_le() {
    roundtrip(PcmFormat::F64(Endianness::Little));
}

#[test]
fn pcm_f64_be() {
    roundtrip(PcmFormat::F64(Endianness::Big));
}
