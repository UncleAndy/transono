use rubato::{Fft, FixedSync};
use rubato::audioadapter_buffers::owned::SequentialOwned;
use crate::audio::SampleBuffer;
use crate::core::error::{CoreError, Result};

const CHANNELS: usize = 1;
const SUB_CHUNKS: usize = 4;

struct ResamplerConfig {
    sample_rate: u32,
    channels: usize,
    chunk_size: usize,
}

pub struct RubatoResampler {
    target_rate: u32,

    config: Option<ResamplerConfig>,

    fft: Option<Fft<f32>>,

    input_buffer: SampleBuffer<f32>,
    output_buffer: SampleBuffer<f32>,

    fft_input: SequentialOwned<f32>,
    fft_output: SequentialOwned<f32>,

    scratch: Vec<f32>,
}

impl RubatoResampler {
    pub fn new(
        output_sample_rate: u32,
    ) -> Self {
        Self {
            output_sample_rate,
            config: None,
            resampler: None,
        }
    }
}
