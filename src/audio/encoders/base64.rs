use base64_simd::{FromBase64Decode, FromBase64Encode};

use crate::core::error::{CoreError, Result};
use crate::audio::{AudioDecoder, AudioEncoder, EncodedAudio, EncodedAudioFormat, PcmAudio};
use crate::audio::encoders::{PcmBinaryDecoder, PcmBinaryEncoder};

pub struct PcmBase64Encoder {
    binary: PcmBinaryEncoder,
}

impl PcmBase64Encoder {
    pub(crate) fn new(format: &EncodedAudioFormat) -> Self {
        Self {
            binary: PcmBinaryEncoder::new(format)
        }
    }
}

impl AudioEncoder for PcmBase64Encoder {
    fn format(&self) -> &EncodedAudioFormat {
        &self.binary.format()
    }

    fn encode(
        &self,
        pcm: &PcmAudio,
    ) -> Result<EncodedAudio> {

        let mut encoded = Vec::new();

        self.encode_bytes(
            pcm,
            &mut encoded,
        )?;

        Ok(EncodedAudio::new(
            self.format().clone(),
            encoded.into(),
        ))
    }

    fn encode_bytes(
        &self,
        pcm: &PcmAudio,
        output: &mut Vec<u8>,
    ) -> Result<()> {
        let mut binary = Vec::new();

        self.binary.encode_bytes(
            pcm,
            &mut binary,
        )?;

        *output = Vec::from_base64_encode(
            &base64_simd::STANDARD,
            &binary,
        );

        Ok(())
    }
}

pub struct PcmBase64Decoder {
    binary: PcmBinaryDecoder
}

impl PcmBase64Decoder {
    pub(crate) fn new(format: &EncodedAudioFormat) -> Self {
        Self {
            binary: PcmBinaryDecoder::new(format)
        }
    }
}

impl AudioDecoder for PcmBase64Decoder {
    fn format(&self) -> &EncodedAudioFormat {
        &self.binary.format()
    }

    fn decode(
        &self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio> {

        self.decode_bytes(
            encoded.bytes(),
        )
    }
    
    fn decode_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<PcmAudio> {

        let binary =
            Vec::from_base64_decode(
                &base64_simd::STANDARD,
                bytes,
            )
                .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

        self.binary.decode_bytes(&binary)
    }
}
