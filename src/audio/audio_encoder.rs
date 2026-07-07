use crate::core::error::Result;
use crate::audio::{EncodedAudio, EncodedAudioFormat, PcmAudio};

trait AudioEncoder {
    fn format(&self) -> &EncodedAudioFormat;
    fn encode(
        &self,
        audio: &PcmAudio,
    ) -> Result<EncodedAudio>;
}

trait AudioDecoder {
    fn format(&self) -> &EncodedAudioFormat;
    fn decode(
        &self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio>;
}
