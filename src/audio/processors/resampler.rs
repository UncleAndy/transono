use rubato::{
    Fft,
    FixedSync,
    audioadapter_buffers::owned::SequentialOwned};
use rubato::Resampler as _;
use symphonia::core::audio::AudioSpec;

use crate::audio::{DspProcessor, PcmAudio, PlanarSampleBuffer};
use crate::core::error::{CoreError, Result};

const SUB_CHUNKS: usize = 4;

pub struct Resampler {
    fft: Fft<f32>,

    input_buffer: PlanarSampleBuffer<f32>,
    output_buffer: PlanarSampleBuffer<f32>,

    fft_input: SequentialOwned<f32>,
    fft_output: SequentialOwned<f32>,
}

impl Resampler {
    pub fn new(
        input_spec: AudioSpec,
        output_rate: u32,
    ) -> Result<Self> {
        let channels = input_spec.channels().count();

        let chunk_size =
            (input_spec.rate() / 50) as usize;

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
        todo!()
    }

    fn process_fft(
        &mut self,
    ) -> Result<()> {
        todo!()
    }

    fn pop_output(
        &mut self,
        pcm: &mut PcmAudio,
    ) {
        todo!()
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
