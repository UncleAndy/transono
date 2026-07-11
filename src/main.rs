use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use std::sync::Arc;
use std::process::Command;
use std::time::Duration;
use symphonia::core::audio::{AudioSpec, Channels, Position};

use realtime_translator::audio::processors::channel_converter::ChannelConverter;
use realtime_translator::audio::processors::resampler::Resampler;
use realtime_translator::audio::{
    AudioDevicesCpal, AudioInput, AudioInputCpal, AudioOutput, AudioOutputCpal, Processor,
    VirtualDevices,
};
use realtime_translator::providers::openai::translation::{
    OpenAITranslationConfig, OpenAITranslationProvider,
};
use realtime_translator::runtime::TranslationLine;
use realtime_translator::console::ConsoleApp;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls provider");

    VirtualDevices::cleanup()?;

    let (virtual_devices, virtual_output_name, virtual_input_name) =
        if let Ok(virtual_devices) = VirtualDevices::create("EN") {
            virtual_devices
        } else {
            panic!("Error creating virtual devices.")
        };

    println!("Waiting...");

    loop {
        let out = Command::new("wpctl").arg("status").output()?;

        let text = String::from_utf8_lossy(&out.stdout);

        if text.contains("Translator") {
            break;
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    println!("Virtual devices created:");
    println!("    input: {}", &virtual_input_name);
    println!("    output: {}", &virtual_output_name);

    let devices = AudioDevicesCpal::new();

    let capture = devices.default_input()?;
    let playback = devices.default_output()?;

    let host = devices.host();

    let to_microphone = host
        .output_devices()?
        .find(|d| {
            d.to_string().contains(&virtual_output_name)
                && d.description().unwrap().supports_output()
        })
        .expect("virtual microphone not found");

    let from_speaker = host
        .input_devices()?
        .find(|d| {
            d.to_string().contains(&virtual_input_name) && d.description().unwrap().supports_input()
        })
        .expect("virtual speaker not found");

    println!("Translator App");
    println!("===========================");
    println!();

    let config = OpenAITranslationConfig::from_env()?.with_lang("en").clone();

    let remote = config.audio_format();

    let remote_spec = remote.spec().clone();

    let mono = Channels::Positioned(Position::FRONT_CENTER);
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

    let stats_direct = Arc::new(realtime_translator::audio::LatencyStats::default());

    let input_hw = AudioInputCpal::new(capture, stats_direct.clone())?;
    let to_microphone_virt = AudioOutputCpal::new(to_microphone, stats_direct.clone())?;

    /*
       let link_format = AudioFormat {
           sample_rate: output_sample_rate,
           channels: stereo.count() as u16,
           sample_format: PcmFormat::F32(Endianness::Little),
       };

       // Для тестирования - Link виртуального микрофона с виртуальным динамиком
       let (link_input, link_output) =
           AudioLink::new_ports(link_format, 32);
    */

    /*
        ------------------------------------------------------------------
        Линия для перевода с реального микрофона на виртуальный (RU -> EN)
        ------------------------------------------------------------------
    */

    // TranslationLine

    // Для отладки
    //let mut line =
    //    TranslationLine::new(provider, Box::new(input_hw), Box::new(link_input)).await?;

    let mut line =
        TranslationLine::new(provider, Box::new(input_hw), Box::new(to_microphone_virt), stats_direct).await?;

    // Input DSP
    {
        line.add_input_processor(Processor::ChannelConverter(ChannelConverter::new(
            mono.clone(),
        )))?;

        line.add_input_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(input_sample_rate, mono.clone()),
            remote_spec.rate(),
        )?))?;
    }

    // Output DSP
    {
        line.add_output_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(remote_spec.rate(), mono.clone()),
            output_sample_rate,
        )?))?;

        line.add_output_processor(Processor::ChannelConverter(ChannelConverter::new(
            stereo.clone(),
        )))?;
    }

    /*
        -----------------------------------------------------------------
        Линия для перевода с виртуального динамика на реальный (EN -> RU)
        -----------------------------------------------------------------
    */

    let config_back = OpenAITranslationConfig::from_env()?.with_lang("ru").clone();
    let remote_back = config_back.audio_format();
    let remote_back_spec = remote_back.spec().clone();
    let provider_back = OpenAITranslationProvider::new(config_back);

    let stats_back = Arc::new(realtime_translator::audio::LatencyStats::default());

    let from_speaker_virt = AudioInputCpal::new(from_speaker, stats_back.clone())?;
    let output_hw = AudioOutputCpal::new(playback, stats_back.clone())?;

    let input_back_sample_rate = from_speaker_virt.format().sample_rate;
    let output_back_sample_rate = output_hw.format().sample_rate;

    // TranslationLine

    // Для отладки - выход линка на вход Line
    // let mut line_back =
    //    TranslationLine::new(provider_back, Box::new(link_output), Box::new(output_hw)).await?;

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
            mono.clone(),
        )))?;

        line_back.add_input_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(input_back_sample_rate, mono.clone()),
            remote_back_spec.rate(),
        )?))?;
    }

    // Output DSP
    {
        line_back.add_output_processor(Processor::Resampler(Resampler::new(
            AudioSpec::new(remote_back_spec.rate(), mono.clone()),
            output_back_sample_rate,
        )?))?;

        line_back.add_output_processor(Processor::ChannelConverter(ChannelConverter::new(
            stereo.clone(),
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

    println!("\nRemove virtual devices...");
    drop(virtual_devices);

    VirtualDevices::cleanup()?;

    println!("Done.");

    Ok(())
}

fn print_latency_stats(snapshot: realtime_translator::audio::LatencySnapshot) {
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

fn print_metric(name: &str, m: realtime_translator::audio::MetricSnapshot) {
    println!(
        "{} | {:6.2} | {:6.2} | {:6.2} | {:6.2} |",
        name, m.min_ms, m.avg_ms, m.max_ms, m.last_ms
    );
}
