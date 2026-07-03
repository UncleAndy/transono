use anyhow::{bail, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize,
    Device,
    SampleFormat,
    Stream,
    StreamConfig,
};

use crate::audio::audio_buffer::FrameProducer;

pub struct AudioCapture {
    stream: Stream,
}

impl AudioCapture {
    pub fn new(
        device: Device,
        mut capture: FrameProducer,
    ) -> Result<Self> {
        let config = select_config(&device)?;

        let stream = device.build_input_stream::<f32, _, _>(
            config,
            move |data: &[f32], _| {
                if let Some(id) = capture.acquire() {
                    if capture.write(id, data) {
                        let _ = capture.commit(id);
                    }
                }
            },
            move |err| {
                eprintln!("capture: {err}");
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
        for device in host.input_devices()? {
            if device.to_string() == name {
                return Ok(device);
            }
        }

        bail!("Input device '{name}' not found");
    }

    host.default_input_device()
        .context("Default input device not found")
}

fn select_config(device: &Device) -> Result<StreamConfig> {
    let cfg = device.default_input_config()?;

    if cfg.sample_format() != SampleFormat::F32 {
        bail!("Input device must support f32");
    }

    Ok(StreamConfig {
        channels: cfg.channels(),
        sample_rate: 48_000,
        buffer_size: BufferSize::Default,
    })
}
