use anyhow::{bail, Result};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize,
    Device,
    SampleFormat,
    Stream,
    StreamConfig,
};

use crate::audio::audio_buffer::FrameConsumer;

pub struct AudioPlayback {
    stream: Stream,
}

impl AudioPlayback {
    pub fn new(
        device: Device,
        mut playback: FrameConsumer,
    ) -> Result<Self> {
        let config = select_config(&device)?;

        println!(
            "Playback: rate={} channels={} buffer={:?}",
            config.sample_rate.to_string(),
            config.channels.to_string(),
            config.buffer_size,
        );

        let stream = device.build_output_stream::<f32, _, _>(
            config,
            move |output: &mut [f32], _| {
                output.fill(0.0);

                if let Some(id) = playback.receive() {
                    let frames = output.len() / 2;

                    let mut mono = vec![0.0f32; frames];

                    playback.copy_from_frame(id, &mut mono);

                    for (stereo, sample) in output.chunks_exact_mut(2).zip(mono.iter()) {
                        stereo[0] = *sample;
                        stereo[1] = *sample;
                    }

                    let _ = playback.release(id);
                }
            },
            move |err| {
                eprintln!("playback: {err}");
            },
            None,
        )?;

        Ok(Self { stream })
    }

    #[inline]
    pub fn start(&self) -> Result<()> {
        self.stream.play()?;
        Ok(())
    }

    #[inline]
    pub fn stop(&self) -> Result<()> {
        self.stream.pause()?;
        Ok(())
    }
}

fn select_config(device: &Device) -> Result<StreamConfig> {
    let cfg = device.default_output_config()?;

    if cfg.sample_format() != SampleFormat::F32 {
        bail!("Output device must support f32");
    }

    Ok(StreamConfig {
        channels: cfg.channels(),
        sample_rate: 48_000,
        buffer_size: BufferSize::Default,
    })
}
