use anyhow::Result;
use cpal::traits::DeviceTrait;
use tokio::signal;
use realtime_translator::audio::{
    AudioDevices,
};
use realtime_translator::providers::openai::realtime::{
    OpenAIRealtimeConfig,
    OpenAIRealtimeProvider,
};

use realtime_translator::runtime::TranslationLine;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls provider");

    let devices = AudioDevices::new();
    let capture = devices.default_input()?;
    let playback = devices.default_output()?;

    println!("Realtime Translator Example");
    println!("===========================");
    println!();

    println!("Input device : {}", capture.description()?);
    println!("Output device: {}", playback.description()?);

    let config = OpenAIRealtimeConfig::from_env()?;

    let remote = config.audio_format();

    println!(
        "OpenAI format: {} Hz, {} channel(s)",
        remote.spec().rate(),
        remote.spec().channels().count(),
    );

    println!();
    println!("Connecting...");

    let provider = OpenAIRealtimeProvider::new(config);

    let mut line =
        TranslationLine::new(
            provider,
            capture,
            playback,
        )
            .await?;

    line.run().await?;

    println!("Connected.");
    println!("Press Ctrl+C to stop.");

    signal::ctrl_c().await?;

    println!("Stopping...");

    line.stop().await?;

    println!("Done.");

    Ok(())
}
