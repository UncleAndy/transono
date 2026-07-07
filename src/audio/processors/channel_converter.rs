use symphonia::core::audio::Channels;

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::{CoreError, Result};

pub struct ChannelConverter {
    output_channels: Channels,

    scratch: Vec<f32>,
}

impl ChannelConverter {
    pub fn new(output_channels: Channels) -> Self {
        Self {
            output_channels,
            scratch: Vec::new(),
        }
    }

    fn stereo_to_mono(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = pcm.frame_count();

        self.scratch.clear();
        self.scratch.resize(frames, 0.0);

        let left = pcm.channel(0);
        let right = pcm.channel(1);

        for i in 0..frames {
            self.scratch[i] = (left[i] + right[i]) * 0.5;
        }

        pcm.replace_channel(
            0,
            &self.scratch,
        );

        pcm.remove_channel(1);
    }

    fn mono_to_stereo(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        self.scratch.clear();
        self.scratch.extend_from_slice(
            pcm.channel(0),
        );

        pcm.add_channel(
            &self.scratch,
        );
    }
}

impl DspProcessor for ChannelConverter {
    fn process(
        &mut self,
        pcm: &mut PcmAudio,
    ) -> Result<()> {

        let input = pcm.channel_count();
        let output = self.output_channels.count();

        match (input, output) {

            (1, 1) | (2, 2) => {
                // ничего делать не нужно
            }

            (2, 1) => {
                self.stereo_to_mono(pcm);
            }

            (1, 2) => {
                self.mono_to_stereo(pcm);
            }

            _ => {
                return Err(
                    CoreError::Other(anyhow::anyhow!(
                        "Unsupported channel conversion: {} -> {}",
                        input,
                        output,
                    ))
                );
            }
        }

        pcm.spec.channels = self.output_channels.clone();

        Ok(())
    }
}
