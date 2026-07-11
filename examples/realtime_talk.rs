use anyhow::Result;
use cpal::traits::DeviceTrait;
use std::sync::Arc;
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio::signal;

use libereco::audio::{AudioDevicesCpal, AudioInputCpal, AudioOutputCpal, LatencyStats, Processor};
use libereco::audio::processors::channel_converter::ChannelConverter;
use libereco::audio::processors::resampler::Resampler;
use libereco::providers::openai::realtime::{
    OpenAIRealtimeConfig,
    OpenAIRealtimeProvider,
};
use libereco::runtime::TranslationLine;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls provider");

    let devices = AudioDevicesCpal::new();

    let capture = devices.default_input()?;
    let playback = devices.default_output()?;

    println!("Realtime Example");
    println!("===========================");
    println!();

    println!("Input device : {}", capture.description()?);
    println!("Input format: {:?}", capture.default_input_config()?.sample_format());
    println!("Output device: {}", playback.description()?);
    println!("Output format: {:?}", playback.default_output_config()?.sample_format());

    let config = OpenAIRealtimeConfig::from_env()?
        .with_voice("marin")
        .clone();

    let remote = config.audio_format();

    let remote_spec = remote.spec().clone();
    let mono = Channels::Positioned(Position::FRONT_CENTER);
    let stereo = Channels::Positioned(
        Position::FRONT_LEFT | Position::FRONT_RIGHT,
    );

    println!(
        "OpenAI format: {} Hz, {} channel(s)",
        remote.spec().rate(),
        remote.spec().channels().count(),
    );

    println!();
    println!("Connecting...");

    let provider = OpenAIRealtimeProvider::new(config);

    let input_sample_rate = capture.default_input_config()?.sample_rate();
    let output_sample_rate = playback.default_output_config()?.sample_rate();

    println!("Capture: {} Hz", input_sample_rate);
    println!("Remote : {} Hz", remote_spec.rate());
    println!("Playback: {} Hz", output_sample_rate);

    let stats = Arc::new(LatencyStats::default());

    let input = AudioInputCpal::new(capture, stats.clone())?;
    let output = AudioOutputCpal::new(playback, stats.clone())?;

    // TranslationLine
    let mut line =
        TranslationLine::new(
            provider,
            Box::new(input),
            Box::new(output),
            stats,
        ).await?;

    // Input DSP
    {
        line.add_input_processor(
            Processor::ChannelConverter(
                ChannelConverter::new(mono.clone())
            )
        )?;

        line.add_input_processor(
            Processor::Resampler(
                Resampler::new(
                    AudioSpec::new(
                        input_sample_rate,
                        mono.clone(),
                    ),
                    remote_spec.rate()
                )?
            )
        )?;
    }

    // Output DSP
    {
        line.add_output_processor(
            Processor::Resampler(
                Resampler::new(
                    AudioSpec::new(
                        remote_spec.rate(),
                        mono.clone(),
                    ),
                    output_sample_rate,
                )?
            )
        )?;

        line.add_output_processor(
            Processor::ChannelConverter(
                ChannelConverter::new(stereo.clone())
            )
        )?;
    }

    println!("Run...");

    line.run().await?;

    println!("Connected.");
    println!("Press Ctrl+C to stop.");

    signal::ctrl_c().await?;

    println!("Stopping...");

    line.stop().await?;

    let latency = line.latency();
    print_latency_stats(latency);

    println!("Done.");

    Ok(())
}

fn print_latency_stats(snapshot: libereco::audio::LatencySnapshot) {
    println!();
    println!("Latency Statistics (ms):");
    println!("---------------------------------------------------------------");
    println!("Stage             | Min    | Avg    | Max    | Last   |");
    println!("---------------------------------------------------------------");
    print_metric("Input Pipeline ", snapshot.input_pipeline);
    print_metric("Input Total    ", snapshot.input_total);
    print_metric("Output Pipeline", snapshot.output_pipeline);
    print_metric("Output Total   ", snapshot.output_total);
    println!("---------------------------------------------------------------");
    println!("Dropped: Input: {}, Network: {}, Output: {}", 
             snapshot.dropped_input, snapshot.dropped_network, snapshot.dropped_output);
    println!("---------------------------------------------------------------");
}

fn print_metric(name: &str, m: libereco::audio::MetricSnapshot) {
    println!(
        "{} | {:6.2} | {:6.2} | {:6.2} | {:6.2} |",
        name, m.min_ms, m.avg_ms, m.max_ms, m.last_ms
    );
}
