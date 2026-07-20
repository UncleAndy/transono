use crate::core::error::{CoreError, Result};
use crate::audio::{AudioCodec, AudioContainer, BinaryEncoding, EncodedAudio, EncodedAudioFormat, PcmAudio};
use crate::audio::encoders::{PcmBase64Decoder, PcmBase64Encoder, PcmBinaryDecoder};
use crate::audio::encoders::PcmBinaryEncoder;

/// Trait for audio encoders.
pub trait AudioEncoder: Send {
    /// Returns the format this encoder produces.
    fn format(&self) -> &EncodedAudioFormat;
    /// Encodes PCM audio into an `EncodedAudio` object.
    fn encode(
        &mut self,
        audio: &PcmAudio,
    ) -> Result<EncodedAudio>;
    /// Encodes PCM audio directly into a byte buffer.
    fn encode_bytes(
        &mut self,
        pcm: &PcmAudio,
        output: &mut Vec<u8>,
    ) -> Result<()>;
}

/// Trait for audio decoders.
pub trait AudioDecoder: Send {
    /// Returns the format this decoder expects.
    fn format(&self) -> &EncodedAudioFormat;
    /// Decodes an `EncodedAudio` object into PCM audio.
    fn decode(
        &mut self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio>;
    /// Decodes raw bytes into PCM audio.
    fn decode_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<PcmAudio>;
}

/// Factory for creating audio encoders and decoders.
pub struct AudioCodecs;

impl AudioCodecs {
    /// Returns an encoder for the specified format.
    pub fn encoder(
        format: &EncodedAudioFormat,
    ) -> Result<Box<dyn AudioEncoder + Send>> {
        match (
            format.container(),
            format.codec(),
            format.encoding(),
        ) {
            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Binary,
            ) => Ok(Box::new(
                PcmBinaryEncoder::new(format)?,
            )),

            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Base64,
            ) => Ok(Box::new(
                PcmBase64Encoder::new(format)?,
            )),

            _ => Err(CoreError::UnsupportedAudioFormat(
                format.clone(),
            )),
        }
    }

    /// Returns a decoder for the specified format.
    pub fn decoder(
        format: &EncodedAudioFormat,
    ) -> Result<Box<dyn AudioDecoder + Send>> {
        match (
            format.container(),
            format.codec(),
            format.encoding(),
        ) {
            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Binary,
            ) => Ok(Box::new(
                PcmBinaryDecoder::new(format)?,
            )),

            (
                AudioContainer::Raw,
                AudioCodec::Pcm(_),
                BinaryEncoding::Base64,
            ) => Ok(Box::new(
                PcmBase64Decoder::new(format)?,
            )),

            _ => Err(CoreError::UnsupportedAudioFormat(
                format.clone(),
            )),
        }
    }
}
