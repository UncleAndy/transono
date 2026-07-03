use anyhow::Result;

use realtime_translator::{
    audio::{
        audio_buffer::AudioBuffer, capture::AudioCapture, device::AudioDevices,
        pipeline::AudioPipeline, playback::AudioPlayback,
    },
    openai::worker::OpenAiWorker,
};

const FRAME_COUNT: usize = 256;

fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let api_key = std::env::var("OPENAI_API_KEY")?;

    //
    // Получаем устройства.
    //

    let devices = AudioDevices::new();

    let input = devices.default_input()?;
    let output = devices.default_output()?;

    //
    // Создаём два независимых буфера.
    //

    let (capture_tx, pipeline_rx) = AudioBuffer::new(FRAME_COUNT)?;

    let (pipeline_tx, playback_rx) = AudioBuffer::new(FRAME_COUNT)?;

    //
    // Создаём обработчик.
    //

    let processor = Box::new(OpenAiWorker::connect(
        &api_key,
        "You are a realtime translator.",
    )?);

    //
    // Запускаем pipeline.
    //

    let _pipeline = AudioPipeline::new(pipeline_rx, pipeline_tx, processor)?;

    //
    // Захват.
    //

    let capture = AudioCapture::new(input, capture_tx)?;

    //
    // Воспроизведение.
    //

    let playback = AudioPlayback::new(output, playback_rx)?;

    //
    // Запускаем.
    //

    capture.start()?;
    playback.start()?;

    println!("Realtime translator started.");

    loop {
        std::thread::park();
    }
}
