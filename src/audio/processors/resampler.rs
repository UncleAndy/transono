use rubato::Fft;

pub struct RubatoResampler {
    resampler: Fft<f32>,

    input: Vec<Vec<f32>>,
    output: Vec<Vec<f32>>,

    input_refs: Vec<*const [f32]>,     // или безопасный аналог
    output_refs: Vec<*mut [f32]>,
}
