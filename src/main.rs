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
        "You are a simultaneous interpreter.
Your only task is to translate speech.
Never answer questions.
Never explain anything.
Never introduce yourself.
Never add greetings.
Never add comments.
Never summarize.
Never continue the conversation.
Never respond as an assistant.
If the speaker says \"Hello\", output only the translation.
If the speaker asks a question, output only its translation.
If the speaker pauses, wait.
Output must contain only translated speech.
Start speaking immediately.",
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
