use crate::core::error::Result;
use crate::audio::{AudioCodec, AudioDecoder, AudioEncoder, EncodedAudio, EncodedAudioFormat, Endianness, PcmAudio};

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

    fn encode(
        &mut self,
        audio: &PcmAudio,
    ) -> Result<EncodedAudio> {
        let mut data = Vec::new();

        self.encode_bytes(audio, &mut data)?;

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
        output.clear();

        let frames = pcm.frames();
        let channels = pcm.channel_count();

        output.resize(frames * channels * size_of::<i16>(), 0);

        let mut pos = 0;
        let little_endian = matches!(
            self.format.codec(),
            AudioCodec::Pcm(Endianness::Little)
        );

        for channel in pcm.channels() {
            for &sample in channel {
                let sample =
                    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;

                let bytes = if little_endian {
                    sample.to_le_bytes()
                } else {
                    sample.to_be_bytes()
                };

                output[pos] = bytes[0];
                output[pos + 1] = bytes[1];

                pos += 2;
            }
        }

        Ok(())
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

    fn decode(
        &mut self,
        encoded: &EncodedAudio,
    ) -> Result<PcmAudio> {
        self.decode_bytes(encoded.bytes())
    }

    fn decode_bytes(
        &mut self,
        bytes: &[u8],
    ) -> Result<PcmAudio> {

        let channels = self.format.spec().channels().count();

        let samples = bytes.len() / 2;

        debug_assert_eq!(samples % channels, 0);

        let frames = samples / channels;

        let mut pcm = PcmAudio::new(
            self.format.spec().clone(),
            frames,
        );

        match self.format.codec() {

            AudioCodec::Pcm(Endianness::Little) => {

                let mut offset = 0;

                for channel in pcm.channels_mut() {
                    for sample in channel {

                        let value = i16::from_le_bytes([
                            bytes[offset],
                            bytes[offset + 1],
                        ]);

                        *sample =
                            value as f32 / i16::MAX as f32;

                        offset += 2;
                    }
                }
            }

            AudioCodec::Pcm(Endianness::Big) => {

                let mut offset = 0;

                for channel in pcm.channels_mut() {
                    for sample in channel {

                        let value = i16::from_be_bytes([
                            bytes[offset],
                            bytes[offset + 1],
                        ]);

                        *sample =
                            value as f32 / i16::MAX as f32;

                        offset += 2;
                    }
                }
            }

            _ => unreachable!(),
        }

        Ok(pcm)
    }
}
