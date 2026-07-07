use crate::core::error::Result;
use crate::audio::{AudioDecoder, AudioEncoder, EncodedAudio, EncodedAudioFormat, PcmAudio};

pub struct PcmBinaryEncoder {
    format: EncodedAudioFormat
}

impl PcmBinaryEncoder {
    pub(crate) fn new(format: &EncodedAudioFormat) -> Self {
        Self {
            format: format.clone()
        }
    }
}

impl AudioEncoder for PcmBinaryEncoder {
    fn format(&self) -> &EncodedAudioFormat {
        &self.format
    }

    fn encode(&self, audio: &PcmAudio) -> Result<EncodedAudio> {
        todo!()
    }
}

pub struct PcmBinaryDecoder {
    format: EncodedAudioFormat
}

impl PcmBinaryDecoder {
    pub(crate) fn new(format: &EncodedAudioFormat) -> Self {
        Self {
            format: format.clone()
        }
    }
}

impl AudioDecoder for PcmBinaryDecoder {
    fn format(&self) -> &EncodedAudioFormat {
        &self.format
    }

    fn decode(&self, encoded: &EncodedAudio) -> Result<PcmAudio> {
        todo!()
    }
}
