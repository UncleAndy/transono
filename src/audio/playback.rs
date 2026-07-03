use anyhow::{bail, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
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

        let stream = device.build_output_stream::<f32, _, _>(
            config,
            move |output: &mut [f32], _| {
                output.fill(0.0);

                if let Some(id) = playback.receive() {
                    playback.copy_from_frame(id, output);
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

fn select_device(
    host: &cpal::Host,
    wanted: Option<&str>,
) -> Result<Device> {
    if let Some(name) = wanted {
        for device in host.output_devices()? {
            if device.to_string() == name {
                return Ok(device);
            }
        }

        bail!("Output device '{name}' not found");
    }

    host.default_output_device()
        .context("Default output device not found")
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
