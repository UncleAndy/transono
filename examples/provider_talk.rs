use anyhow::Result;
use tokio::sync::mpsc::Receiver;
use realtime_translator::audio::{
    AudioBuffer,
    AudioCapture,
    AudioDevices,
    AudioPlayback,
    FrameConsumer,
    FrameProducer,
    RubatoResampler,
    FRAME_CAPACITY,
};
use realtime_translator::core::provider::Provider;
use realtime_translator::providers::openai::realtime::{
    OpenAIRealtimeConfig,
    OpenAIRealtimeProvider,
    RealtimeSession,
    TurnMode,
};
use realtime_translator::audio::{
    base64_to_pcm16,
    pcm16_to_base64,
};
use realtime_translator::providers::openai::realtime::commands::{
    InputAudioBufferAppend,
};

use realtime_translator::providers::openai::realtime::events::ProtocolEvent;

const CAPTURE_BUFFER_SIZE: usize = 256;
const PLAYBACK_BUFFER_SIZE: usize = 256;

#[tokio::main]
async fn main() -> Result<()> {

    dotenvy::dotenv().ok();

    println!("Creating provider...");

    let config = OpenAIRealtimeConfig::from_env()?
        .with_model("gpt-realtime")
        .with_voice("cedar")
        .with_turn_mode(TurnMode::ServerVad)
        .with_instructions(
            "You are a friendly conversational assistant. \
             Speak briefly and naturally."
        ).clone();

    let (audio_tx, audio_rx) =
        tokio::sync::mpsc::channel::<Vec<f32>>(32);

    let provider = OpenAIRealtimeProvider::new(config.clone());

    println!("Opening session...");

    let session = provider.create_session().await?;

    println!("Creating audio...");

    let devices = AudioDevices::new();

    let input = devices.default_input()?;
    let output = devices.default_output()?;

    let (capture_tx, capture_rx) =
        AudioBuffer::new(CAPTURE_BUFFER_SIZE)?;

    let (playback_tx, playback_rx) =
        AudioBuffer::new(PLAYBACK_BUFFER_SIZE)?;

    let capture =
        AudioCapture::new(
            input,
            capture_tx,
        )?;

    let playback =
        AudioPlayback::new(
            output,
            playback_rx,
        )?;

    capture.start()?;
    playback.start()?;

    let capture_tx = audio_tx.clone();

    let capture_thread = std::thread::spawn(move || {
        capture_forwarder(
            capture_rx,
            capture_tx,
        );
    });

    println!();
    println!("======================================");
    println!(" OpenAI Realtime Talk Demo");
    println!("======================================");
    println!();
    println!("Speak into your microphone.");
    println!("Press Ctrl+C to exit.");
    println!();

    let session_task = tokio::spawn(
        realtime_task(
            session,
            audio_rx,
            playback_tx,
        )
    );

    tokio::signal::ctrl_c().await?;

    println!("\nStopping...");

    println!("Stopping capture...");
    capture.stop()?;
    println!("Stopping playback...");
    playback.stop()?;

    // Сначала убиваем async-задачу
    session_task.abort();
    let _ = session_task.await;

    // Теперь закрываем последний Sender
    drop(audio_tx);

    // Теперь поток сам завершится
    capture_thread.join().unwrap();

    println!("Done.");
    Ok(())
}

fn capture_forwarder(
    mut capture: FrameConsumer,
    tx: tokio::sync::mpsc::Sender<Vec<f32>>,
) {
    loop {
        if tx.is_closed() {
            break;
        }

        if let Some(id) = capture.receive() {

            let samples =
                capture.read(id, |frame| {
                    frame.samples().to_vec()
                });

            if tx.blocking_send(samples).is_err() {
                break;
            }

            capture.release(id).unwrap();
        } else {
            std::thread::yield_now();
        }
    }
}

async fn realtime_task(
    mut session: RealtimeSession,
    mut audio_rx: Receiver<Vec<f32>>,
    mut playback: FrameProducer,
) -> Result<()> {

    let mut input_resampler = RubatoResampler::new()?;
    let mut output_resampler = RubatoResampler::new()?;

    let mut input_pcm = Vec::<i16>::new();
    let mut output_float = Vec::<f32>::new();

    loop {

        tokio::select! {

            Some(samples) = audio_rx.recv() => {
                input_pcm.clear();

                input_resampler.in_processor(
                    &samples,
                    &mut input_pcm,
                )?;

                if !input_pcm.is_empty() {
                    let base64 = pcm16_to_base64(&input_pcm);

                    session.send(
                        InputAudioBufferAppend::new(base64)
                    ).await?;
                }
            }

            event = session.next_event() => {

                match event? {

                    ProtocolEvent::ResponseOutputAudioDelta { delta } => {
                        let pcm16 =
                            base64_to_pcm16(&delta)?;

                        output_float.clear();

                        output_resampler.out_processor(
                            &pcm16,
                            &mut output_float,
                        )?;

                        for chunk in output_float.chunks(FRAME_CAPACITY) {
                            playback.send(chunk);
                        }
                    }

                    ProtocolEvent::InputAudioBufferSpeechStarted => {
                        println!("🎤 Speech");
                    }

                    ProtocolEvent::ResponseCreated => {
                        println!("🤖 Thinking...");
                    }

                    ProtocolEvent::ResponseDone => {
                        println!("✅ Done");
                    }

                    ProtocolEvent::Error { error } => {
                        println!("OpenAI: {}", error.message);
                    }

                    _ => {}
                }
            }
        }
    }
}
