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

        self.binary.encode_bytes(
            pcm,
            &mut self.scratch,
        )?;

        output.clear();

        output.resize(
            base64_simd::STANDARD
                .encoded_length(self.scratch.len()),
            0,
        );

        let _ = base64_simd::STANDARD.encode(
            &self.scratch,
            base64_simd::Out::from_slice(output),
        );

        Ok(())
    }
}

pub struct PcmBase64Decoder {
    format: EncodedAudioFormat,
    binary: PcmBinaryDecoder,
    scratch: Vec<u8>,
}

impl PcmBase64Decoder {
    pub(crate) fn new(format: &EncodedAudioFormat) -> Self {
        Self {
            format: format.clone(),
            binary: PcmBinaryDecoder::new(format),
            scratch: Vec::new(),
        }
    }
}

impl AudioDecoder for PcmBase64Decoder {
    fn format(&self) -> &EncodedAudioFormat {
        &self.format
    }

    fn decode(
        &mut self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio> {
        self.decode_bytes(encoded.bytes())
    }

    fn decode_bytes(
        &mut self,
        input: &[u8],
    ) -> Result<PcmAudio> {

        self.scratch.clear();

        let decoded_len = base64_simd::STANDARD
            .decoded_length(input)
            .map_err(|e| CoreError::Other(e.into()))?;

        self.scratch.resize(decoded_len, 0);

        let decoded = base64_simd::STANDARD
            .decode(
                input,
                base64_simd::Out::from_slice(&mut self.scratch),
            )
            .map_err(|e| CoreError::Other(e.into()))?;

        self.binary.decode_bytes(decoded)
    }
}
