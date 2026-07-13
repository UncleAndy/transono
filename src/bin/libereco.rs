use anyhow::{Context, Result, anyhow};
use cpal::{
    Device, Host,
    traits::{DeviceTrait, HostTrait},
};
use libereco::audio::diagnost::indicator::Indicator;
use libereco::audio::processors::channel_converter::ChannelConverter;
use libereco::audio::processors::denoiser::Denoiser;
use libereco::audio::processors::resampler::Resampler;
use libereco::audio::{AudioDevicesCpal, AudioInput, AudioInputCpal, AudioOutput, AudioOutputCpal, EncodedAudioFormat, Processor};
use libereco::console::ConsoleApp;
use libereco::ctl::create_backend;
use libereco::providers::openai::translation::{
    OpenAITranslationConfig, OpenAITranslationProvider,
};
use libereco::runtime::TranslationLine;
use std::sync::Arc;
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio::sync::mpsc;
use libereco::audio::processors::compressor::{Compressor, NATURAL_VOICE};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls provider");

    let language = "en";
    let language_self = "ru";

    let backend = create_backend().context("failed to create audio control backend")?;
    let virtual_devices = backend
        .devices(language)
        .context("failed to resolve virtual audio device names")?;

    println!("Virtual devices:");
    println!(
        "    meeting microphone: {}",
        &virtual_devices.to_meeting_microphone
    );
    println!(
        "    meeting speaker   : {}",
        &virtual_devices.from_meeting_speaker
    );
    println!(
        "    internal output   : {}",
        &virtual_devices.internal_to_meeting_speaker
    );
    println!(
        "    internal input    : {}",
        &virtual_devices.internal_from_meeting_microphone
    );

    let devices = AudioDevicesCpal::new();

    let capture = devices.default_input()?;
    let playback = devices.default_output()?;

    let host = devices.host();

    let to_microphone =
        find_virtual_output(host, &virtual_devices.internal_to_meeting_speaker, language)?;

    let from_speaker = find_virtual_input(
        host,
        &virtual_devices.internal_from_meeting_microphone,
        language,
    )?;

    println!("Translator App");
    println!("===========================");
    println!();

    let config = OpenAITranslationConfig::from_env()?
        .with_lang(language)
        .clone();

    let remote = config.audio_format();

    let remote_spec = remote.spec().clone();

    let internal_spec = EncodedAudioFormat::internal_format().spec();
    let internal_channels = internal_spec.channels().clone();
    let stereo = Channels::Positioned(Position::FRONT_LEFT | Position::FRONT_RIGHT);

    println!(
        "OpenAI format: {} Hz, {} channel(s)",
        remote.spec().rate(),
        remote.spec().channels().count(),
    );

    println!();
    println!("Connecting...");

    let provider = OpenAITranslationProvider::new(config);

    let output_sample_rate = to_microphone.default_output_config()?.sample_rate();
    let input_sample_rate = capture.default_input_config()?.sample_rate();

    println!("Capture: {} Hz", input_sample_rate);
    println!("Remote : {} Hz", remote_spec.rate());
    println!("Playback: {} Hz", output_sample_rate);

    let stats_direct = Arc::new(libereco::audio::LatencyStats::default());

    let input_hw = AudioInputCpal::new(capture, stats_direct.clone())?;
    let to_microphone_virt = AudioOutputCpal::new(to_microphone, stats_direct.clone())?;

    /*
        ------------------------------------------------------------------
        Линия для перевода с реального микрофона на виртуальный (RU -> EN)
        ------------------------------------------------------------------
    */

    // TranslationLine

    let (direct_input_indicator_tx, direct_input_indicator_rx) = mpsc::channel(8);
    let (direct_output_indicator_tx, direct_output_indicator_rx) = mpsc::channel(8);

    let mut line = TranslationLine::new(
        provider,
        Box::new(input_hw),
        Box::new(to_microphone_virt),
        stats_direct,
    )
    .await?;

    // Input DSP
    {
        line.add_input_processor(Processor::ChannelConverter(ChannelConverter::new(
            internal_channels.clone(),
        )))?;

        line.add_input_processor(Processor::Denoiser(Denoiser::new(AudioSpec::new(
            input_sample_rate,
            internal_channels.clone(),
        ))))?;

        line.add_input_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(input_sample_rate, internal_channels.clone()),
            remote_spec.rate(),
        )?))?;

        line.add_input_processor(Processor::Compressor(Compressor::new(
            NATURAL_VOICE.clone(),
        )))?;

        line.add_input_processor(Processor::IndicatorDiag(Indicator::new(
            direct_input_indicator_tx,
        )))?;
    }

    // Output DSP
    {
        line.add_output_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(remote_spec.rate(), internal_channels.clone()),
            output_sample_rate,
        )?))?;

        line.add_output_processor(Processor::ChannelConverter(ChannelConverter::new(
            stereo.clone(),
        )))?;

        line.add_output_processor(Processor::IndicatorDiag(Indicator::new(
            direct_output_indicator_tx,
        )))?;
    }

    /*
        -----------------------------------------------------------------
        Линия для перевода с виртуального динамика на реальный (EN -> RU)
        -----------------------------------------------------------------
    */

    let config_back = OpenAITranslationConfig::from_env()?
        .with_lang(language_self)
        .clone();
    let remote_back = config_back.audio_format();
    let remote_back_spec = remote_back.spec().clone();
    let provider_back = OpenAITranslationProvider::new(config_back);

    let stats_back = Arc::new(libereco::audio::LatencyStats::default());

    let from_speaker_virt = AudioInputCpal::new(from_speaker, stats_back.clone())?;
    let output_hw = AudioOutputCpal::new(playback, stats_back.clone())?;

    let input_back_sample_rate = from_speaker_virt.format().sample_rate;
    let output_back_sample_rate = output_hw.format().sample_rate;

    // TranslationLine "en" -> "ru"

    let (back_input_indicator_tx, back_input_indicator_rx) = mpsc::channel(8);
    let (back_output_indicator_tx, back_output_indicator_rx) = mpsc::channel(8);

    let mut line_back = TranslationLine::new(
        provider_back,
        Box::new(from_speaker_virt),
        Box::new(output_hw),
        stats_back,
    )
    .await?;

    // Input DSP
    {
        line_back.add_input_processor(Processor::ChannelConverter(ChannelConverter::new(
            internal_channels.clone(),
        )))?;

        line_back.add_input_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(input_back_sample_rate, internal_channels.clone()),
            remote_back_spec.rate(),
        )?))?;

        line_back.add_input_processor(Processor::IndicatorDiag(Indicator::new(
            back_input_indicator_tx,
        )))?;
    }

    // Output DSP
    {
        line_back.add_output_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(remote_back_spec.rate(), internal_channels.clone()),
            output_back_sample_rate,
        )?))?;

        line_back.add_output_processor(Processor::ChannelConverter(ChannelConverter::new(
            stereo.clone(),
        )))?;

        line_back.add_output_processor(Processor::IndicatorDiag(Indicator::new(
            back_output_indicator_tx,
        )))?;
    }

    /*
    ----------------------------------------------------------------------------------
     */

    println!("Run lines...");

    let (direct_tx, direct_rx) = mpsc::unbounded_channel();
    let (back_tx, back_rx) = mpsc::unbounded_channel();

    line.set_event_sender(direct_tx);
    line_back.set_event_sender(back_tx);

    line.run().await?;
    line_back.run().await?;

    let app = ConsoleApp::new(
        direct_rx,
        back_rx,
        line.latency_stats.clone(),
        line_back.latency_stats.clone(),
        direct_input_indicator_rx,
        direct_output_indicator_rx,
        back_input_indicator_rx,
        back_output_indicator_rx,
    );

    println!("Press 'q' to stop.");
    // app.run() blocks until 'q' or Esc
    app.run().await?;

    println!("Stopping...");

    println!("Stop back line...");
    line_back.stop().await?;
    println!("Stop direct line...");
    line.stop().await?;

    println!("\nDirect Line Latency:");
    print_latency_stats(line.latency());
    println!("\nBack Line Latency:");
    print_latency_stats(line_back.latency());

    println!("Done.");

    Ok(())
}

fn find_virtual_output(host: &Host, name: &str, language: &str) -> Result<Device> {
    host.output_devices()
        .context("failed to list output devices")?
        .find(|device| {
            device.to_string().contains(name)
                && device
                    .description()
                    .map(|description| description.supports_output())
                    .unwrap_or(false)
        })
        .ok_or_else(|| missing_virtual_device("output", name, language))
}

fn find_virtual_input(host: &Host, name: &str, language: &str) -> Result<Device> {
    host.input_devices()
        .context("failed to list input devices")?
        .find(|device| {
            device.to_string().contains(name)
                && device
                    .description()
                    .map(|description| description.supports_input())
                    .unwrap_or(false)
        })
        .ok_or_else(|| missing_virtual_device("input", name, language))
}

fn missing_virtual_device(direction: &str, name: &str, language: &str) -> anyhow::Error {
    anyhow!(
        "virtual {direction} device '{name}' not found; run `liberecoctl init {language}` first"
    )
}

fn print_latency_stats(snapshot: libereco::audio::LatencySnapshot) {
    println!("---------------------------------------------------------------");
    println!("Stage             | Min    | Avg    | Max    | Last   |");
    println!("---------------------------------------------------------------");
    print_metric("Input Pipeline ", snapshot.input_pipeline);
    print_metric("Input Total    ", snapshot.input_total);
    print_metric("Output Pipeline", snapshot.output_pipeline);
    print_metric("Output Total   ", snapshot.output_total);
    println!("---------------------------------------------------------------");
    println!(
        "Dropped: Input: {}, Network: {}, Output: {}",
        snapshot.dropped_input, snapshot.dropped_network, snapshot.dropped_output
    );
    println!("---------------------------------------------------------------");
}

fn print_metric(name: &str, m: libereco::audio::MetricSnapshot) {
    println!(
        "{} | {:6.2} | {:6.2} | {:6.2} | {:6.2} |",
        name, m.min_ms, m.avg_ms, m.max_ms, m.last_ms
    );
}
