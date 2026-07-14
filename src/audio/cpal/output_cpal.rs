use std::sync::Arc;
use anyhow::anyhow;
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use crate::audio::{sample_to_pcm_format, Audio, AudioFormat, AudioOutput, LatencyStats};
use crate::core::error::{CoreError, Result};

pub struct AudioOutputCpal {
    #[allow(unused)]
    name: String,
    stream: Stream,
    format: AudioFormat,
    sender: Option<Sender<Audio>>,
    stats: Arc<LatencyStats>,
}

struct OutputStateCpal<T> {
    current: Option<Audio>,
    current_samples: Vec<T>,
    offset: usize,
}

impl Drop for AudioOutputCpal {
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}

impl AudioOutputCpal {
    pub fn new(
        device: Device,
        stats: Arc<LatencyStats>,
    ) -> Result<Self> {
        let (config, sample_format) = select_config(&device)?;

        let (tx, rx) = mpsc::channel(256);

        let stream = match sample_format {
            SampleFormat::F32 => {
                Self::build_stream::<f32>(
                    &device,
                    &config,
                    rx,
                    stats.clone(),
                )?
            }

            SampleFormat::I16 => {
                Self::build_stream::<i16>(
                    &device,
                    &config,
                    rx,
                    stats.clone(),
                )?
            }

            SampleFormat::U16 => {
                Self::build_stream::<u16>(
                    &device,
                    &config,
                    rx,
                    stats.clone(),
                )?
            }

            _ => {
                return Err(CoreError::Other(anyhow::anyhow!(
                "Unsupported sample format"
            )));
            }
        };

        Ok(Self {
            name: device.to_string(),
            stream,
            format: AudioFormat {
                sample_rate: config.sample_rate,
                channels: config.channels,
                sample_format: sample_to_pcm_format(sample_format),
            },
            sender: Some(tx),
            stats,
        })
    }

    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        mut receiver: mpsc::Receiver<Audio>,
        stats: Arc<LatencyStats>,
    ) -> Result<Stream>
    where
        T: cpal::SizedSample
        + symphonia::core::audio::conv::ConvertibleSample
        + Send
        + 'static,
    {
        let mut state = OutputStateCpal {
            current: None,
            current_samples: Vec::<T>::new(),
            offset: 0,
        };

        let stream = device.build_output_stream::<T, _, _>(
            *config,
            move |output: &mut [T], _| {
                output.fill(T::EQUILIBRIUM);

                let mut output_offset = 0;

                while output_offset < output.len() {
                    // Если текущий пакет закончился — взять следующий.
                    if state.current.is_none() {
                        state.current = receiver.try_recv().ok();

                        let Some(audio) = &state.current else {
                            stats.inc_dropped_output();
                            break;
                        };

                        state.current_samples.clear();

                        audio
                            .buffer()
                            .copy_to_vec_interleaved(
                                &mut state.current_samples,
                            );

                        state.offset = 0;
                    }

                    let remain =
                        &state.current_samples[state.offset..];

                    let count = remain
                        .len()
                        .min(output.len() - output_offset);

                    output[
                        output_offset..
                            output_offset + count
                        ]
                        .copy_from_slice(
                            &remain[..count],
                        );

                    output_offset += count;
                    state.offset += count;

                    if state.offset >= state.current_samples.len() {
                        state.current = None;
                        state.current_samples.clear();
                        state.offset = 0;
                    }
                }
            },
            move |err| {
                eprintln!("playback: {err}");
            },
            None,
        )
            .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

        Ok(stream)
    }
}

impl AudioOutput for AudioOutputCpal {
    fn clone_sender(&mut self) -> Result<Sender<Audio>> {
        let Some(sender) = self.sender.clone() else {
            return Err(CoreError::Other(anyhow!("sender absent")))
        };

        Ok(sender)
    }

    #[inline]
    fn start(&self) -> Result<()> {
        self.stream.play().map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;
        Ok(())
    }

    #[inline]
    fn stop(&self) -> Result<()> {
        self.stream.pause().map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;
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
    let cfg = device.default_output_config()
        .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

    Ok((StreamConfig {
        channels: cfg.channels(),
        sample_rate: cfg.sample_rate(),
        buffer_size: BufferSize::Default,
    }, cfg.sample_format()))
}
