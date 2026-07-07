use crate::core::error::{CoreError, Result};
use crate::audio::{AudioCodec, AudioContainer, BinaryEncoding, EncodedAudio, EncodedAudioFormat, PcmAudio};
use crate::audio::encoders::{PcmBase64Decoder, PcmBase64Encoder, PcmBinaryDecoder};
use crate::audio::encoders::PcmBinaryEncoder;

pub trait AudioEncoder {
    fn format(&self) -> &EncodedAudioFormat;
    fn encode(
        &self,
        audio: &PcmAudio,
    ) -> Result<EncodedAudio>;
}

pub trait AudioDecoder {
    fn format(&self) -> &EncodedAudioFormat;
    fn decode(
        &self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio>;
}

pub struct AudioCodecs;

impl AudioCodecs {
    pub fn encoder(
        format: &EncodedAudioFormat,
    ) -> Result<Box<dyn AudioEncoder>> {
        match (
            format.container(),
            format.codec(),
            format.encoding(),
        ) {
            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Binary,
            ) => {
                Ok(Box::new(
                    PcmBinaryEncoder::new(&format)
                ))
            }

            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Base64,
            ) => {
                Ok(Box::new(
                    PcmBase64Encoder::new(&format)
                ))
            }

            _ => Err(CoreError::UnsupportedAudioFormat(
                format.clone(),
            )),
        }
    }

    pub fn decoder(
        format: &EncodedAudioFormat,
    ) -> Result<Box<dyn AudioDecoder>> {
        match (
            format.container(),
            format.codec(),
            format.encoding(),
        ) {
            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Binary,
            ) => {
                Ok(Box::new(
                    PcmBinaryDecoder::new(&format)
                ))
            }

            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Base64,
            ) => {
                Ok(Box::new(
                    PcmBase64Decoder::new(&format)
                ))
            }

            _ => Err(CoreError::UnsupportedAudioFormat(
                format.clone(),
            )),
        }
    }
}
