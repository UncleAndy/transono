use crate::core::error::Result;
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

    fn encode(&self, audio: &PcmAudio) -> Result<EncodedAudio> {
        todo!()
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

    fn decode(&self, encoded: &EncodedAudio) -> Result<PcmAudio> {
        todo!()
    }
}
