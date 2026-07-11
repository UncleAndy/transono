use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::{CoreError, Result};

pub struct WavDump {
    writer: Option<WavWriter<BufWriter<File>>>,
}

impl WavDump {
    pub fn new(
        path: impl AsRef<Path>,
        spec: symphonia::core::audio::AudioSpec,
    ) -> Result<Self> {
        let wav_spec = WavSpec {
            channels: spec.channels().count() as u16,
            sample_rate: spec.rate(),
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let writer = WavWriter::create(path, wav_spec)
            .map_err(|e| CoreError::Other(e.into()))?;

        Ok(Self {
            writer: Some(writer)
        })
    }
}

impl DspProcessor for WavDump {
    fn process(
        &mut self,
        pcm: &mut PcmAudio,
    ) -> Result<bool> {
        let frames = pcm.frames();
        let channels = pcm.channel_count();

        for frame in 0..frames {
            for channel in 0..channels {
                let sample = pcm.channel(channel)[frame];

                let sample = (sample.clamp(-1.0, 1.0)
                    * i16::MAX as f32) as i16;

                self.writer
                    .as_mut()
                    .unwrap()
                    .write_sample(sample)
                    .map_err(|e| CoreError::Other(e.into()))?;
            }
        }

        Ok(true)
    }
}

impl Drop for WavDump {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            let _ = writer.finalize();
        }
    }
}
