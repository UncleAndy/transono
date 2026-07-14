use std::sync::Arc;
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use symphonia::core::audio::{AudioBuffer, AudioSpec, Channels};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use crate::core::error::{CoreError, Result};
use crate::audio::audio::{Audio, AudioFormat, IntoGenericBuffer};
use crate::audio::{AudioInput, LatencyStats};
use crate::audio::cpal::sample_to_pcm_format;

pub struct AudioInputCpal {
    #[allow(unused)]
    name: String,
    stream: Stream,
    format: AudioFormat,
    receiver: Option<Receiver<Audio>>,
    stats: Arc<LatencyStats>,
}

impl Drop for AudioInputCpal {
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}

impl AudioInputCpal {
    pub fn new(
        device: Device,
        stats: Arc<LatencyStats>,
    ) -> Result<Self> {
        let (config, sample_format) = select_config(&device)?;

        let spec = AudioSpec::new(
            config.sample_rate,
            Channels::Discrete(config.channels as u16),
        );

        let format = AudioFormat {
            sample_rate: config.sample_rate,
            channels: config.channels,
            sample_format: sample_to_pcm_format(sample_format),
        };

        let (tx, rx) = mpsc::channel(256);

        let stream = match sample_format {

            SampleFormat::F32 =>
                Self::build_stream::<f32>(
                    &device,
                    &config,
                    spec.clone(),
                    tx,
                    stats.clone(),
                )?,

            SampleFormat::I16 =>
                Self::build_stream::<i16>(
                    &device,
                    &config,
                    spec.clone(),
                    tx,
                    stats.clone(),
                )?,

            SampleFormat::U16 =>
                Self::build_stream::<u16>(
                    &device,
                    &config,
                    spec.clone(),
                    tx,
                    stats.clone(),
                )?,

            _ => {
                return Err(CoreError::Internal(
                    "Unsupported sample format".to_string()
                ));
            }
        };

        Ok(Self {
            name: device.to_string(),
            stream,
            format,
            receiver: Some(rx),
            stats,
        })
    }

    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        spec: AudioSpec,
        sender: mpsc::Sender<Audio>,
        stats: Arc<LatencyStats>,
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
 
                if frames == 0 {
                    return;
                }
 
                let mut buffer =
                    AudioBuffer::<T>::new(
                        spec.clone(),
                        frames,
                    );

                buffer.render_uninit(Some(frames));

                buffer.copy_from_slice_interleaved::<T, &[T]>(&data);

                let audio =
                    Audio::new(buffer.into_generic_buffer());

                if sender.is_closed() {
                    return;
                }

                match sender.try_send(audio) {
                    Ok(_) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        stats.inc_dropped_input();
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => {
                        // линия уже остановлена
                        return;
                    }
                }
            },
            move |err| {
                eprintln!("capture: {err}");
            },
            None,
        )
            .map_err(|e| CoreError::Cpal(e.to_string()))?;

        Ok(stream)
    }
}

impl AudioInput for AudioInputCpal {
    #[inline]
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        let Some(receiver) = self.receiver.take() else {
            return Err(CoreError::Internal("receiver already taken".to_string()))
        };

        Ok(ReceiverStream::new(receiver).boxed())
    }

    #[inline]
    fn start(&self) -> Result<()> {
        self.stream.play().map_err(|e| CoreError::Cpal(e.to_string()))?;
        Ok(())
    }
 
    #[inline]
    fn stop(&self) -> Result<()> {
        self.stream.pause().map_err(|e| CoreError::Cpal(e.to_string()))?;
        Ok(())
    }

    #[inline]
    fn format(&self) -> AudioFormat {
        self.format.clone()
    }

    fn set_stats(&mut self, stats: Arc<LatencyStats>) {
        self.stats = stats;
    }
}

fn select_config(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
    let cfg = device.default_input_config().map_err(|e| CoreError::Cpal(e.to_string()))?;

    Ok((StreamConfig {
        channels: cfg.channels(),
        sample_rate: cfg.sample_rate(),
        buffer_size: BufferSize::Default,
    }, cfg.sample_format()))
}
