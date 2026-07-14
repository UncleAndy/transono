use crate::core::error::{CoreError, Result};
use crate::audio::{AudioCodec, AudioDecoder, AudioEncoder, EncodedAudio, EncodedAudioFormat, PcmAudio, PcmFormat};

pub struct PcmBinaryEncoder {
    format: EncodedAudioFormat,
    pcm_format: PcmFormat,
}

impl PcmBinaryEncoder {
    pub(crate) fn new(
        format: &EncodedAudioFormat,
    ) -> Result<Self> {
        let pcm_format = match format.codec() {
            AudioCodec::Pcm(format) => format,
            _ => {
                return Err(
                    CoreError::UnsupportedAudioFormat(
                        format.clone(),
                    )
                )
            }
        };

        Ok(Self {
            format: format.clone(),
            pcm_format,
        })
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
 
        EncodedAudio::new(
            self.format.clone(),
            data.into(),
        )
    }

    fn encode_bytes(
        &mut self,
        pcm: &PcmAudio,
        output: &mut Vec<u8>,
    ) -> Result<()> {
        let sample_size = self.pcm_format.sample_size();

        output.clear();

        output.resize(
            pcm.frames()
                * pcm.channel_count()
                * sample_size,
            0,
        );

        let mut offset = 0;

        for &sample in &pcm.data {
            self.pcm_format.encode_sample(
                sample,
                &mut output[offset..offset + sample_size],
            );

            offset += sample_size;
        }

        Ok(())
    }
}

pub struct PcmBinaryDecoder {
    format: EncodedAudioFormat,
    pcm_format: PcmFormat,
}

impl PcmBinaryDecoder {
    pub(crate) fn new(
        format: &EncodedAudioFormat,
    ) -> Result<Self> {
        let pcm_format = match format.codec() {
            AudioCodec::Pcm(format) => format,
            _ => {
                return Err(
                    CoreError::UnsupportedAudioFormat(
                        format.clone(),
                    )
                )
            }
        };

        Ok(Self {
            format: format.clone(),
            pcm_format,
        })
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
        let sample_size = self.pcm_format.sample_size();

        let channels = self.format.spec().channels().count();

        let samples = bytes.len() / sample_size;

        debug_assert_eq!(samples % channels, 0);

        let frames = samples / channels;

        let mut pcm = PcmAudio::new(
            self.format.spec().clone(),
            frames,
        );

        let mut offset = 0;

        for sample in &mut pcm.data {
            *sample = self.pcm_format.decode_sample(
                &bytes[offset..offset + sample_size],
            );

            offset += sample_size;
        }

        Ok(pcm)
    }
}
