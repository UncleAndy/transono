use anyhow::Result;
use cpal::traits::DeviceTrait;
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio::signal;
use realtime_translator::audio::{AudioDevicesCpal, AudioInputCpal, AudioOutputCpal, Processor};
use realtime_translator::audio::processors::channel_converter::ChannelConverter;
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

    let config = OpenAIRealtimeConfig::from_env()?;

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

    let input = AudioInputCpal::new(capture)?;
    let output = AudioOutputCpal::new(playback)?;

    // TranslationLine
    let mut line =
        TranslationLine::new(
            provider,
            Box::new(input),
            Box::new(output),
        ).await?;

    // Input DSP
    {
        line.add_input_processor(
            Processor::Dsp(Box::new(
                ChannelConverter::new(mono.clone())
            ))
        )?;

        line.add_input_processor(
            Processor::Dsp(Box::new(
                Resampler::new(
                    AudioSpec::new(
                        input_sample_rate,
                        mono.clone(),
                    ),
                    remote_spec.rate()
                )?
            ))
        )?;
    }

    // Output DSP
    {
        line.add_output_processor(
            Processor::Dsp(Box::new(
                Resampler::new(
                    AudioSpec::new(
                        remote_spec.rate(),
                        mono.clone(),
                    ),
                    output_sample_rate,
                )?
            ))
        )?;

        line.add_output_processor(
            Processor::Dsp(Box::new(
                ChannelConverter::new(stereo.clone())
            ))
        )?;
    }

    println!("Run...");

    line.run().await?;

    println!("Connected.");
    println!("Press Ctrl+C to stop.");

    signal::ctrl_c().await?;

    println!("Stopping...");

    line.stop().await?;

    println!("Done.");

    Ok(())
}
