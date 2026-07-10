use anyhow::anyhow;
use symphonia::core::audio::Channels;

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::{CoreError, Result};

pub struct ChannelConverter {
    output_channels: Channels,
}

impl ChannelConverter {
    pub fn new(
        output_channels: Channels,
    ) -> Self {
        Self {
            output_channels,
        }
    }

    fn stereo_to_mono(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = pcm.frames();

        for i in 0..frames {
            let left = pcm.data[i];
            let right = pcm.data[i + frames];
            pcm.data[i] = (left + right) * 0.5;
        }

        pcm.data.truncate(frames);
        pcm.set_channel_layout(self.output_channels.clone());
    }

    fn mono_to_stereo(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = pcm.frames();
        pcm.data.extend_from_within(0..frames);
        pcm.set_channel_layout(self.output_channels.clone());
    }
}

impl DspProcessor for ChannelConverter {
    fn process(
        &mut self,
        pcm: &mut PcmAudio,
    ) -> Result<()> {

        match (
            pcm.channel_count(),
            self.output_channels.count(),
        ) {
            (1, 1) | (2, 2) => {
                pcm.set_channel_layout(
                    self.output_channels.clone(),
                );
            }

            (2, 1) => {
                self.stereo_to_mono(pcm);
            }

            (1, 2) => {
                self.mono_to_stereo(pcm);
            }

            (from, to) => {
                return Err(CoreError::Other(
                    anyhow!(
                        "unsupported channel conversion: {} -> {}",
                        from,
                        to,
                    ),
                ));
            }
        }

        Ok(())
    }
}
