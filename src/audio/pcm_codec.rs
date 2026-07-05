use crate::core::error::{CoreError, Result};

use crate::audio::{
    Audio,
    AudioCodec,
    AudioEncoding,
    AudioFormat,
    EncodedAudio,
    Endianness,
};

pub struct PcmCodec {
    format: AudioFormat,
    endianness: Endianness,
}

impl PcmCodec {
    pub fn new(
        format: AudioFormat,
        endianness: Endianness,
    ) -> Self {
        Self {
            format,
            endianness,
        }
    }
}

impl AudioCodec for PcmCodec {
    fn encoding(&self) -> AudioEncoding {
        AudioEncoding::Pcm {
            endianness: self.endianness,
        }
    }

    fn encode(
        &self,
        audio: &Audio,
    ) -> Result<EncodedAudio> {

        if audio.format() != &self.format {
            return Err(CoreError::Other(anyhow::format_err!("unsupported audio format")));
        }

        Ok(EncodedAudio::new(
            AudioEncoding::Pcm {
                endianness: self.endianness,
            },
            audio.buffer().clone(),
        ))
    }

    fn decode(
        &self,
        encoded: &EncodedAudio,
    ) -> Result<Audio> {
        match encoded.encoding() {
            AudioEncoding::Pcm { endianness } => {
                if *endianness != self.endianness {
                    return Err(CoreError::Other(anyhow::format_err!("unsupported endianness")));
                }

                Ok(Audio::new(
                    self.format,
                    encoded.bytes().clone(),
                ))
            }

            _ => {
                Err(CoreError::Other(anyhow::format_err!("unsupported encoding")))
            }
        }
    }
}
