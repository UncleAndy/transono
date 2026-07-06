use rubato::{Fft, FixedSync};

use crate::core::error::{CoreError, Result};

const CHANNELS: usize = 1;
const SUB_CHUNKS: usize = 4;

pub struct RubatoResampler {
    resampler: Fft<f32>,
}

impl RubatoResampler {

    pub fn new(
        input_rate: u32,
        output_rate: u32,
        channels: usize,
    ) -> Result<Self> {

        let resampler = Fft::<f32>::new(
            input_rate as usize,
            output_rate as usize,
            1024,
            SUB_CHUNKS,
            channels,
            FixedSync::Input,
        )
            .map_err(|e| CoreError::Other(anyhow::Error::from(e)))?;

        Ok(Self {
            resampler,
        })
    }
}
