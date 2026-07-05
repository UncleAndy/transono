use anyhow::{bail, Result};
use bytes::Bytes;
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use tokio::sync::mpsc;
use crate::audio::{Audio, AudioFormat};

pub struct AudioPlayback {
    stream: Stream,
    format: AudioFormat,
}

struct PlaybackState {
    current: Option<Audio>,
    current_samples: Option<Bytes>,
    offset: usize,
}

impl AudioPlayback {
    pub fn new(
        device: Device,
        mut receiver: mpsc::Receiver<Audio>,
    ) -> Result<Self> {
        let (config, sample_format) = select_config(&device)?;

        println!(
            "Playback: rate={} channels={} buffer={:?}",
            config.sample_rate.to_string(),
            config.channels.to_string(),
            config.buffer_size,
        );

        let mut state = PlaybackState {
            current: None,
            current_samples: None,
            offset: 0,
        };

        let stream = device.build_output_stream::<f32, _, _>(
            config,
            move |output: &mut [f32], _| {

                output.fill(0.0);

                if state.current.is_none() {
                    state.current = receiver.try_recv().ok();
                    state.offset = 0;
                }

                let Some(audio) = &state.current else {
                    return;
                };

                let samples: &[f32] = match audio.view::<f32>() {
                    Ok(samples) => samples,
                    Err(err) => {
                        eprintln!("playback: {err}");
                        state.current = None;
                        return;
                    }
                };

                let remain = &samples[state.offset..];
                let count = remain.len().min(output.len());

                output[..count].copy_from_slice(&remain[..count]);

                state.offset += count;

                if state.offset >= samples.len() {
                    state.current = None;
                    state.offset = 0;
                }
            },
            move |err| {
                eprintln!("playback: {err}");
            },
            None,
        )?;

        Ok(Self {
            stream,
            format: AudioFormat {
                sample_rate: config.sample_rate,
                channels: config.channels,
                sample_format,
            }
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

fn select_config(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
    let cfg = device.default_output_config()?;

    Ok((StreamConfig {
        channels: cfg.channels(),
        sample_rate: 48_000,
        buffer_size: BufferSize::Default,
    }, cfg.sample_format()))
}
