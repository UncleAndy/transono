use std::{
    env,
    fs::{self, File},
    io::Write,
};

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use dotenvy::dotenv;

use realtime_translator::{
    core::provider::Provider,
    providers::openai::realtime::commands::{
        InputAudioBufferAppend,
        InputAudioBufferCommit,
        ResponseCreate,
        SessionUpdate,
    },
    providers::openai::realtime::config::OpenAIRealtimeConfig,
    providers::openai::realtime::provider::OpenAIRealtimeProvider,
    providers::openai::realtime::events::ProtocolEvent,
};
use realtime_translator::providers::openai::realtime::config::TurnMode;

const CHUNK_SIZE: usize = 16 * 1024;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key = env::var("OPENAI_API_KEY")?;

    println!("Creating provider...");

    let config = OpenAIRealtimeConfig {
        api_key,
        model: "gpt-realtime".into(),
        endpoint: "wss://api.openai.com/v1/realtime".into(),

        organization: None,
        project: None,

        headers: Default::default(),

        turn_mode: TurnMode::Manual,

        instructions: Some("Ты - собеседник для ведения интересных разговоров.".to_string()),

        voice: Some("cedar".to_string()),
    };

    let provider = OpenAIRealtimeProvider::new(config.clone());

    println!("Opening session...");

    let mut session = provider.create_session().await?;

    println!("Updating session...");

    session.send(
        SessionUpdate::new(config.session())
    ).await?;

    //
    // Ждем подтверждения обновления сессии.
    //
    loop {
        match session.next_event().await? {
            ProtocolEvent::SessionCreated { .. } => {
                println!("Session created.");
            }

            ProtocolEvent::SessionUpdated { .. } => {
                println!("Session updated.");
                break;
            }

            event => {
                println!("{event:#?}");
            }
        }
    }

    println!("Reading sample.pcm...");

    let pcm = fs::read("sample_16_24000_mono.raw")?;

    let total_chunks = pcm.chunks(CHUNK_SIZE).count();

    println!(
        "Uploading {} bytes ({} chunks)...",
        pcm.len(),
        total_chunks
    );

    for (index, chunk) in pcm.chunks(CHUNK_SIZE).enumerate() {
        println!(
            "Sending chunk {}/{}",
            index + 1,
            total_chunks
        );

        session.send(
            InputAudioBufferAppend::new(
                BASE64.encode(chunk),
            )
        ).await?;
    }

    println!("Committing audio...");

    session.send(
        InputAudioBufferCommit::new(),
    ).await?;

    println!("Creating response...");

    session.send(
        ResponseCreate::new(),
    ).await?;

    println!("Receiving audio...");

    let mut output = File::create("output.pcm")?;

    let mut total = 0usize;

    loop {
        match session.next_event().await? {

            ProtocolEvent::ResponseOutputAudioDelta { delta } => {
                let pcm = BASE64.decode(delta)?;

                total += pcm.len();

                output.write_all(&pcm)?;

                println!(
                    "Received {} bytes (total {})",
                    pcm.len(),
                    total,
                );
            }

            ProtocolEvent::ResponseOutputAudioDone => {
                println!("Audio finished.");
            }

            ProtocolEvent::ResponseDone => {
                println!("Response finished.");
                break;
            }

            event => {
                println!("{event:#?}");
            }
        }
    }

    println!(
        "Saved {} bytes to output.pcm",
        total,
    );

    Ok(())
}
