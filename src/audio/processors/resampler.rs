use rubato::Resampler as _;
use rubato::{
    Fft,
    FixedSync,
    audioadapter_buffers::owned::SequentialOwned};
use rubato::audioadapter::{Adapter, AdapterMut};
use symphonia::core::audio::AudioSpec;

use crate::audio::{DspProcessor, PcmAudio, PlanarSampleBuffer};
use crate::core::error::{CoreError, Result};

const SUB_CHUNKS: usize = 4;
const CHUNK_DURATION_MS: u32 = 20;

pub struct Resampler {
    output_rate: u32,

    fft: Fft<f32>,

    input_buffer: PlanarSampleBuffer<f32>,
    output_buffer: PlanarSampleBuffer<f32>,

    fft_input: SequentialOwned<f32>,
    fft_output: SequentialOwned<f32>,

    channels_scratch: Vec<Vec<f32>>,
}

impl Resampler {
    pub fn new(
        input_spec: AudioSpec,
        output_rate: u32,
    ) -> Result<Self> {
        let channels = input_spec.channels().count();

        let chunk_size = frames_from_duration(input_spec.rate(), CHUNK_DURATION_MS);

        let fft = Fft::<f32>::new(
            input_spec.rate() as usize,
            output_rate as usize,
            chunk_size,
            SUB_CHUNKS,
            channels,
            FixedSync::Input,
        )
            .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

        let (fft_input, fft_output) = Self::create_fft_buffers(
            &fft,
            channels
        );

        Ok(Self {
            output_rate,
            channels_scratch: (0..channels)
                .map(|_| Vec::with_capacity(fft.output_frames_max()))
                .collect(),
            input_buffer: PlanarSampleBuffer::new(channels),
            output_buffer: PlanarSampleBuffer::new(channels),
            fft_input,
            fft_output,
            fft,
        })
    }

    fn create_fft_buffers(
        fft: &Fft<f32>,
        channels: usize,
    ) -> (
        SequentialOwned<f32>,
        SequentialOwned<f32>,
    ) {
        (
            SequentialOwned::new(
                0.0,
                channels,
                fft.input_frames_max(),
            ),
            SequentialOwned::new(
                0.0,
                channels,
                fft.output_frames_max(),
            )
        )
    }

    fn push_input(
        &mut self,
        pcm: &PcmAudio,
    ) {
        let channels_count = pcm.channel_count();

        debug_assert_eq!(
            channels_count,
            self.input_buffer.channels(),
        );
        for channel in 0..channels_count {
            self.input_buffer.push_channel(
                channel,
                pcm.channel(channel),
            );
        }
    }
    fn process_fft(
        &mut self,
    ) -> Result<()> {
        let channels_count = self.input_buffer.channels();

        loop {
            let required = self.fft.input_frames_next();

            if self.input_buffer.available() < required {
                break;
            }

            // ---------- input ----------
            for channel in 0..channels_count {
                let samples = self
                    .input_buffer
                    .read_channel(channel, required)
                    .unwrap();

                self.fft_input.copy_from_slice_to_channel(
                    channel,
                    0,
                    samples,
                );
            }

            println!(
                "need={}, inbuf={}",
                required,
                self.input_buffer.available(),
            );

            let (input_frames, output_frames) = self
                .fft
                .process_into_buffer(
                    &self.fft_input,
                    &mut self.fft_output,
                    None,
                )
                .map_err(|e| CoreError::Other(e.into()))?;

            println!(
                "fft in={} out={} outbuf={}",
                input_frames,
                output_frames,
                self.output_buffer.available(),
            );

            // ---------- output ----------
            for channel in 0..channels_count {
                let scratch = &mut self.channels_scratch[channel];

                scratch.resize(output_frames, 0.0);

                self.fft_output.copy_from_channel_to_slice(
                    channel,
                    0,
                    scratch,
                );

                self.output_buffer.push_channel(
                    channel,
                    scratch,
                );
            }

            self.input_buffer.consume(input_frames);

            println!(
                "after pop={}",
                self.output_buffer.available(),
            );
        }

        Ok(())
    }

    fn pop_output(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        let frames = self.output_buffer.available();

        if frames == 0 {
            return;
        }

        let channels = pcm.channel_count();

        debug_assert_eq!(
            channels,
            self.output_buffer.channels(),
        );

        let mut output = Vec::with_capacity(channels);

        for channel in 0..channels {
            let samples = self
                .output_buffer
                .read_channel(channel, frames)
                .unwrap();

            output.push(samples.to_vec());
        }

        self.output_buffer.consume(frames);

        pcm.replace_channels(
            output,
            pcm.spec.channels().clone(),
        );

        if pcm.spec.rate() != self.output_rate {
            pcm.spec = AudioSpec::new(
                self.output_rate,
                pcm.spec.channels().clone(),
            );
        }
    }
}

impl DspProcessor for Resampler {
    fn process(&mut self, input: &mut PcmAudio) -> Result<()> {
        self.push_input(input);

        self.process_fft()?;

        self.pop_output(input);

        Ok(())
    }
}

fn frames_from_duration(
    sample_rate: u32,
    duration_ms: u32,
) -> usize {
    (sample_rate as usize * duration_ms as usize) / 1000
}
