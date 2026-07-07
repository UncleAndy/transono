use anyhow::anyhow;
use symphonia::core::audio::Channels;

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::{CoreError, Result};

pub struct ChannelConverter {
    output_channels: Channels,
    scratch: Vec<f32>,
}

impl ChannelConverter {
    pub fn new(
        output_channels: Channels,
    ) -> Self {
        Self {
            output_channels,
            scratch: Vec::new(),
        }
    }

    fn stereo_to_mono(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = pcm.frames();

        self.scratch.clear();
        self.scratch.resize(frames, 0.0);

        let left = pcm.channel(0);
        let right = pcm.channel(1);

        for i in 0..frames {
            self.scratch[i] =
                (left[i] + right[i]) * 0.5;
        }

        pcm.replace_channels(
            vec![std::mem::take(&mut self.scratch)],
            self.output_channels.clone(),
        );
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
            self.output_channels.clone(),
        );
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
