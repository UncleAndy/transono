use bytes::Bytes;
use cpal::SampleFormat;

use crate::audio::{Audio, AudioFormat, AudioProcessor};
use crate::core::error::{CoreError, Result};

pub struct SampleConverter {
    from: SampleFormat,
    to: SampleFormat,
}

impl SampleConverter {
    pub fn new(from: SampleFormat, to: SampleFormat) -> Self {
        Self { from, to }
    }

    fn f32_to_i16(
        &self,
        audio: Audio,
    ) -> Result<Audio> {

        let samples: &[f32] = audio.view()
            .map_err(|e| CoreError::Other(anyhow::Error::msg(e.to_string())))?;

        let mut out = Vec::<i16>::with_capacity(samples.len());

        for &s in samples {

            out.push(
                (s.clamp(-1.0, 1.0) * i16::MAX as f32)
                    .round() as i16
            );

        }

        Ok(Audio::new(
            AudioFormat {
                sample_format: SampleFormat::I16,
                ..audio.format().clone()
            },
            Bytes::copy_from_slice(
                bytemuck::cast_slice(&out)
            ),
        ))
    }

    fn i16_to_f32(
        &self,
        audio: Audio,
    ) -> Result<Audio> {
        let samples: &[i16] = audio.view()
            .map_err(|e| CoreError::Other(anyhow::Error::msg(e.to_string())))?;

        let mut out = Vec::<f32>::with_capacity(samples.len());

        for &s in samples {

            out.push(
                s as f32 / i16::MAX as f32
            );

        }

        Ok(Audio::new(
            AudioFormat {
                sample_format: SampleFormat::F32,
                ..audio.format().clone()
            },
            Bytes::copy_from_slice(
                bytemuck::cast_slice(&out)
            ),
        ))
    }
}

impl AudioProcessor for SampleConverter {
    fn process(
        &mut self,
        audio: Audio,
    ) -> Result<Audio> {

        if self.from == self.to {
            return Ok(audio);
        }

        match (self.from, self.to) {

            (SampleFormat::F32, SampleFormat::I16) => {
                self.f32_to_i16(audio)
            }

            (SampleFormat::I16, SampleFormat::F32) => {
                self.i16_to_f32(audio)
            }

            _ => Err("Unsupported sample conversion".into()),
        }
    }
}
