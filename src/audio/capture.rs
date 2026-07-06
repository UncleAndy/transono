use anyhow::anyhow;
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use symphonia::core::audio::{AudioBuffer, AudioSpec, Channels};
use tokio::sync::mpsc;

use crate::core::error::{CoreError, Result};
use crate::audio::audio::{Audio, AudioFormat, IntoGenericBuffer};

pub struct AudioCapture {
    stream: Stream,
    format: AudioFormat,
}

impl AudioCapture {
    pub fn new(
        device: Device,
        sender: mpsc::Sender<Audio>,
    ) -> Result<Self> {
        let (config, sample_format) = select_config(&device)?;

        let spec = AudioSpec::new(
            config.sample_rate,
            Channels::Discrete(config.channels as u16),
        );

        let format = AudioFormat {
            sample_rate: config.sample_rate,
            channels: config.channels,
            sample_format,
        };

        println!(
            "Capture: rate={} channels={} buffer={:?}",
            config.sample_rate.to_string(),
            config.channels.to_string(),
            config.buffer_size,
        );

        let stream = match sample_format {

            SampleFormat::F32 =>
                Self::build_stream::<f32>(
                    &device,
                    &config,
                    spec.clone(),
                    sender,
                )?,

            SampleFormat::I16 =>
                Self::build_stream::<i16>(
                    &device,
                    &config,
                    spec.clone(),
                    sender,
                )?,

            SampleFormat::U16 =>
                Self::build_stream::<u16>(
                    &device,
                    &config,
                    spec.clone(),
                    sender,
                )?,

            _ => {
                return Err(CoreError::Other(anyhow!(
                    "Unsupported sample format"
                )));
            }
        };

        Ok(Self {
            stream,
            format,
        })
    }

    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        spec: AudioSpec,
        sender: mpsc::Sender<Audio>,
    ) -> Result<Stream>
    where
        T: cpal::SizedSample
            + symphonia::core::audio::conv::ConvertibleSample
            + symphonia::core::audio::conv::FromSample<T>
            + Send
            + 'static,
        AudioBuffer<T>: IntoGenericBuffer,
    {
        use symphonia::core::audio::{AudioBuffer, AudioMut};

        let stream = device.build_input_stream::<T, _, _>(
            *config,
            move |data: &[T], _| {

                let frames =
                    data.len() / spec.channels().count();

                let mut buffer =
                    AudioBuffer::<T>::new(
                        spec.clone(),
                        frames,
                    );

                buffer.render_uninit(Some(frames));

                buffer.copy_from_slice_interleaved::<T, &[T]>(&data);

                let audio =
                    Audio::new(buffer.into_generic_buffer());

                let _ =
                    sender.blocking_send(audio);

            },
            move |err| {
                eprintln!("capture: {err}");
            },
            None,
        )
            .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

        Ok(stream)
    }

    #[inline]
    pub fn start(&self) -> Result<()> {
        self.stream.play().map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;
        Ok(())
    }

    #[inline]
    pub fn stop(&self) -> Result<()> {
        self.stream.pause().map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;
        Ok(())
    }

    #[inline]
    pub fn format(&self) -> &AudioFormat {
        &self.format
    }
}

fn select_config(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
    let cfg = device.default_input_config().map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

    Ok((StreamConfig {
        channels: cfg.channels(),
        sample_rate: 48_000,
        buffer_size: BufferSize::Default,
    }, cfg.sample_format()))
}
