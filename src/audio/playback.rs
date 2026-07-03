use anyhow::{bail, Result};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};

use crate::audio::{audio_buffer::FrameConsumer, frame::FrameId};

pub struct AudioPlayback {
    stream: Stream,
}

struct PlaybackState {
    current_frame: Option<FrameId>,
    offset: usize,
    mono: Vec<f32>,
}

impl AudioPlayback {
    pub fn new(device: Device, mut playback: FrameConsumer) -> Result<Self> {
        let config = select_config(&device)?;

        println!(
            "Playback: rate={} channels={} buffer={:?}",
            config.sample_rate.to_string(),
            config.channels.to_string(),
            config.buffer_size,
        );

        let mut state = PlaybackState {
            current_frame: None,
            offset: 0,
            mono: Vec::new(),
        };

        let stream = device.build_output_stream::<f32, _, _>(
            config,
            move |output: &mut [f32], _| {
                output.fill(0.0);

                let frames = output.len() / 2;

                //
                // Небольшой рабочий буфер.
                // Пока оставляем Vec, потом уберём аллокацию.
                //
                if state.mono.len() != frames {
                    state.mono.resize(frames, 0.0);
                }

                if state.current_frame.is_none() {
                    state.current_frame = playback.receive();
                    state.offset = 0;
                }

                if let Some(id) = state.current_frame {
                    let finished = playback.read_frame(id, &mut state.offset, &mut state.mono);

                    for (stereo, sample) in output.chunks_exact_mut(2).zip(state.mono.iter()) {
                        stereo[0] = *sample;
                        stereo[1] = *sample;
                    }

                    if finished {
                        let _ = playback.release(id);
                        state.current_frame = None;
                        state.offset = 0;
                    }
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
