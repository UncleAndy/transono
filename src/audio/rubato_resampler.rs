use anyhow::Result;

use rubato::{
    audioadapter::{Adapter, AdapterMut},
    audioadapter_buffers::owned::SequentialOwned,
    Fft, FixedSync, Resampler,
};

use crate::audio::resampler::buffer::SampleBuffer;

const INPUT_RATE: usize = 48_000;
const OUTPUT_RATE: usize = 24_000;

const CHANNELS: usize = 1;
const SUB_CHUNKS: usize = 4;

pub struct RubatoResampler {
    fft_in: Fft<f32>,
    fft_out: Fft<f32>,

    in_input: SampleBuffer<f32>,
    in_output: SampleBuffer<f32>,

    out_input: SampleBuffer<f32>,
    out_output: SampleBuffer<f32>,

    fft_in_input: SequentialOwned<f32>,
    fft_in_output: SequentialOwned<f32>,

    fft_out_input: SequentialOwned<f32>,
    fft_out_output: SequentialOwned<f32>,

    copy_scratch: Vec<f32>,
}

impl RubatoResampler {
    pub fn new() -> Result<Self> {
        let fft_in = Fft::<f32>::new(
            INPUT_RATE,
            OUTPUT_RATE,
            1024,
            SUB_CHUNKS,
            CHANNELS,
            FixedSync::Input,
        )?;

        let fft_out = Fft::<f32>::new(
            OUTPUT_RATE,
            INPUT_RATE,
            512,
            SUB_CHUNKS,
            CHANNELS,
            FixedSync::Input,
        )?;

        let fft_in_input = SequentialOwned::new(0.0, CHANNELS, fft_in.input_frames_max());

        let fft_in_output = SequentialOwned::new(0.0, CHANNELS, fft_in.output_frames_max());

        let fft_out_input = SequentialOwned::new(0.0, CHANNELS, fft_out.input_frames_max());

        let fft_out_output = SequentialOwned::new(0.0, CHANNELS, fft_out.output_frames_max());

        Ok(Self {
            fft_in,
            fft_out,

            in_input: SampleBuffer::new(),
            in_output: SampleBuffer::new(),

            out_input: SampleBuffer::new(),
            out_output: SampleBuffer::new(),

            fft_in_input,
            fft_in_output,

            fft_out_input,
            fft_out_output,

            copy_scratch: Vec::new(),
        })
    }

    /// 48 kHz f32 -> 24 kHz i16.
    pub fn in_processor(&mut self, input: &[f32], output: &mut Vec<i16>) -> Result<()> {
        output.clear();
        self.in_input.push(input);

        while let Some(chunk) = self.in_input.read(self.fft_in.input_frames_next()) {
            self.fft_in_input.copy_from_slice_to_channel(0, 0, chunk);

            let (input_frames, output_frames) = self.fft_in.process_into_buffer(
                &self.fft_in_input,
                &mut self.fft_in_output,
                None,
            )?;

            self.copy_scratch.resize(output_frames, 0.0);
            self.fft_in_output
                .copy_from_channel_to_slice(0, 0, &mut self.copy_scratch);
            self.in_output.push(&self.copy_scratch);
            self.in_input.consume(input_frames);
        }

        if self.in_output.is_empty() {
            return Ok(());
        }

        self.copy_scratch.resize(self.in_output.available(), 0.0);
        self.in_output
            .read(self.copy_scratch.len())
            .expect("available output must be readable")
            .iter()
            .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .for_each(|sample| output.push(sample));
        self.in_output.consume(self.copy_scratch.len());

        Ok(())
    }

    /// 24 kHz i16 -> 48 kHz f32.
    pub fn out_processor(&mut self, input: &[i16], output: &mut Vec<f32>) -> Result<()> {
        output.clear();

        self.copy_scratch.clear();
        self.copy_scratch.reserve(input.len());
        self.copy_scratch
            .extend(input.iter().map(|sample| *sample as f32 / i16::MAX as f32));
        self.out_input.push(&self.copy_scratch);

        while let Some(chunk) = self.out_input.read(self.fft_out.input_frames_next()) {
            self.fft_out_input.copy_from_slice_to_channel(0, 0, chunk);

            let (input_frames, output_frames) = self.fft_out.process_into_buffer(
                &self.fft_out_input,
                &mut self.fft_out_output,
                None,
            )?;

            self.copy_scratch.resize(output_frames, 0.0);
            self.fft_out_output
                .copy_from_channel_to_slice(0, 0, &mut self.copy_scratch);
            self.out_output.push(&self.copy_scratch);
            self.out_input.consume(input_frames);
        }

        if let Some(samples) = self.out_output.read(self.out_output.available()) {
            output.extend_from_slice(samples);
            self.out_output.consume(samples.len());
        }

        Ok(())
    }
}
