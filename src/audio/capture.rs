use anyhow::{bail, Result};
use bytes::Bytes;
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use symphonia::core::audio::{AudioSpec, GenericAudioBuffer};
use symphonia::core::audio::conv::ConvertibleSample;
use tokio::sync::mpsc;
use crate::audio::{Audio, AudioFormat};

pub struct AudioCapture {
    stream: Stream,
    format: AudioFormat,
}

impl AudioCapture {
    pub fn new(
        device: Device,
        sender: mpsc::Sender<Audio>,
    ) -> Result<Self> {
        let config = select_config(&device)?;

        let spec = AudioSpec::new(
            config.sample_rate,
            config.channels.cast_signed(),
        );

        let format = AudioFormat {
            sample_rate: config.sample_rate,
            channels: config.channels,
            sample_format: SampleFormat::F32,
        };

        println!(
            "Capture: rate={} channels={} buffer={:?}",
            config.sample_rate.to_string(),
            config.channels.to_string(),
            config.buffer_size,
        );

        let stream = device.build_input_stream::<f32, _, _>(
            config,
            move |data: &[f32], _| {
                let audio = Audio::from_interleaved(
                    spec.clone(),
                    data,
                );

                let _ = sender.blocking_send(audio);
            },
            move |err| {
                eprintln!("capture: {err}");
            },
            None,
        )?;

        Ok(Self {
            stream,
            format,
        })
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

    #[inline]
    pub fn format(&self) -> &AudioFormat {
        &self.format
    }
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
