use anyhow::Result;
use cpal::traits::DeviceTrait;
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio::signal;
use realtime_translator::audio::{AudioDevices, Processor};
use realtime_translator::audio::processors::resampler::Resampler;
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
            capture.clone(),
            playback.clone(),
        )
            .await?;

    let input_sample_rate = capture.default_input_config()?.sample_rate();
    let output_sample_rate = playback.default_output_config()?.sample_rate();

    line.add_input_processor(
        Processor::Dsp(Box::new(
            Resampler::new(
                AudioSpec::new(
                    input_sample_rate,
                    Channels::Positioned(Position::FRONT_CENTER)
                ),
                remote.spec().rate()
            )?
        ))
    )?;

    line.add_output_processor(
        Processor::Dsp(Box::new(
            Resampler::new(
                AudioSpec::new(
                    remote.spec().rate(),
                    Channels::Positioned(Position::FRONT_CENTER)
                ),
                output_sample_rate,
            )?
        ))
    )?;

    line.run().await?;

    println!("Connected.");
    println!("Press Ctrl+C to stop.");

    signal::ctrl_c().await?;

    println!("Stopping...");

    line.stop().await?;

    println!("Done.");

    Ok(())
}
