use anyhow::{bail, Result};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize,
    Device,
    SampleFormat,
    Stream,
    StreamConfig,
};

use crate::audio::{
    audio_buffer::FrameConsumer,
    frame::FrameId,
};

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

        let mut current_frame: Option<FrameId> = None;
        let mut offset = 0usize;

        let stream = device.build_output_stream::<f32, _, _>(
            config,
            move |output: &mut [f32], _| {
                output.fill(0.0);

                let frames = output.len() / 2;

                //
                // Небольшой рабочий буфер.
                // Пока оставляем Vec, потом уберём аллокацию.
                //
                let mut mono = vec![0.0f32; frames];

                if current_frame.is_none() {
                    current_frame = playback.receive();
                    offset = 0;
                }

                if let Some(id) = current_frame {

                    let finished = playback.read_frame(
                        id,
                        &mut offset,
                        &mut mono,
                    );

                    for (stereo, sample) in output
                        .chunks_exact_mut(2)
                        .zip(mono.iter())
                    {
                        stereo[0] = *sample;
                        stereo[1] = *sample;
                    }

                    if finished {
                        let _ = playback.release(id);
                        current_frame = None;
                        offset = 0;
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
