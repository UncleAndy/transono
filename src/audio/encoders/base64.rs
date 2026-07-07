use base64_simd::{FromBase64Decode, FromBase64Encode};

use crate::core::error::{CoreError, Result};
use crate::audio::{AudioDecoder, AudioEncoder, EncodedAudio, EncodedAudioFormat, PcmAudio};
use crate::audio::encoders::{PcmBinaryDecoder, PcmBinaryEncoder};

pub struct PcmBase64Encoder {
    format: EncodedAudioFormat,
    binary: PcmBinaryEncoder,
    scratch: Vec<u8>,
}

impl PcmBase64Encoder {
    pub(crate) fn new(format: &EncodedAudioFormat) -> Self {
        Self {
            format: format.clone(),
            binary: PcmBinaryEncoder::new(format),
            scratch: Vec::new(),
        }
    }
}

impl AudioEncoder for PcmBase64Encoder {
    fn format(&self) -> &EncodedAudioFormat {
        &self.format
    }

    fn encode(
        &mut self,
        pcm: &PcmAudio,
    ) -> Result<EncodedAudio> {

        let mut data = Vec::new();

        self.encode_bytes(pcm, &mut data)?;

        Ok(EncodedAudio::new(
            self.format.clone(),
            data.into(),
        ))
    }

    fn encode_bytes(
        &mut self,
        pcm: &PcmAudio,
        output: &mut Vec<u8>,
    ) -> Result<()> {

        // Получаем бинарный PCM в переиспользуемый буфер.
        self.binary.encode_bytes(
            pcm,
            &mut self.scratch,
        )?;

        // Кодируем в Base64 сразу в выходной буфер.
        output.clear();

        *output = Vec::from_base64_encode(
            &base64_simd::STANDARD,
            &self.scratch,
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
        &mut self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio> {
        self.decode_bytes(
            encoded.bytes(),
        )
    }

    fn decode_bytes(
        &mut self,
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
