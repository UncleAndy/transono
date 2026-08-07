use anyhow::{Context, Result, anyhow};
use cpal::{
    Device, DeviceId, Host,
    traits::{DeviceTrait, HostTrait},
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;

use transono::audio::diagnost::indicator::Indicator;
use transono::audio::processors::compressor::{Compressor, NATURAL_VOICE};
use transono::audio::processors::denoiser::Denoiser;
use transono::audio::{
    AudioDevicesCpal, AudioFormat, AudioInput, AudioInputCpal, AudioOutputCpal, Processor,
};
use transono::console::ConsoleApp;
use transono::core::provider::Provider;
use transono::ctl::create_backend;
use transono::line::TranslationLine;
use transono::providers::openai::translation::{
    OpenAITranslationConfig, OpenAITranslationProvider,
};
use transono::runtime::{AudioLink, AudioMixer, AudioSplitter};

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
        &virtual_devices.to_meeting_microphone_in
    );
    println!(
        "    meeting speaker   : {}",
        &virtual_devices.from_meeting_speaker_out
    );
    println!(
        "    internal output   : {}",
        &virtual_devices.internal_to_meeting_speaker_out
    );
    println!(
        "    internal input    : {}",
        &virtual_devices.internal_from_meeting_microphone_in
    );

    let devices = AudioDevicesCpal::new();

    let capture = devices.default_input()?;
    let playback = devices.default_output()?;

    let host = devices.host();

    println!(
        "Check virtual output: {}",
        &virtual_devices.internal_to_meeting_speaker_out
    );
    let to_microphone = find_virtual_output(
        host,
        &virtual_devices.internal_to_meeting_speaker_out,
        language,
    )?;
    println!("{:#?}", to_microphone.default_output_config());

    println!(
        "Check virtual input: {}",
        &virtual_devices.internal_from_meeting_microphone_in
    );
    let from_speaker = find_virtual_input(
        host,
        &virtual_devices.internal_from_meeting_microphone_in,
        language,
    )?;
    println!("{:#?}", from_speaker.default_input_config());

    println!("Translator App");
    println!("===========================");
    println!();

    let config = OpenAITranslationConfig::from_env()?
        .with_lang(language)
        .clone();

    let remote_spec = config.audio_format().spec().clone();

    println!(
        "OpenAI format: {} Hz, {} channel(s)",
        remote_spec.rate(),
        remote_spec.channels().count(),
    );

    println!();
    println!("Connecting...");

    let provider = OpenAITranslationProvider::new(config);

    let output_sample_rate = to_microphone.default_output_config()?.sample_rate();
    let input_sample_rate = capture.default_input_config()?.sample_rate();

    println!("Capture: {} Hz", input_sample_rate);
    println!("Remote : {} Hz", remote_spec.rate());
    println!("Playback: {} Hz", output_sample_rate);

    let stats_direct = Arc::new(transono::audio::LatencyStats::default());

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

    // Сплиттер пока с одной линией для отладки
    let mut splitter = AudioSplitter::new(input_hw.format(), 32, Box::new(input_hw));
    let output_for_translate = splitter.create_output();
    // Второй выход сплиттера — оригинальный голос для мониторинга на той стороне
    let mut original_out = splitter.create_output();

    // Микшер, для которого создаем входы, а выход направляем на виртуальный микрофон
    let mixer = AudioMixer::new(output_for_translate.format());
    // Создаем линк для передачи данных из line в микшер
    let (to_mixer_sender, mut to_mixer_receiver) =
        AudioLink::new_ports(output_for_translate.format(), 32);
    // Добавляем в микшер вход из линка от line (перевод на полной громкости)
    let direct_translate_ch = mixer.add_input(&mut to_mixer_receiver, 1.0)?;
    // Параллельный приглушённый канал оригинального голоса (0.5)
    let direct_original_ch = mixer.add_input(original_out.as_mut(), 0.5)?;
    // Выход микшера как отдельный порт, соединяем с виртуальным микрофоном
    let mixer_out = mixer.get_output();
    let _link_from_mixer_to_virt_mic = AudioLink::new_link(
        output_for_translate.format(),
        32,
        Box::new(mixer_out),
        Box::new(to_microphone_virt),
    );
    // Запускаем фоновый цикл микшера (spawn внутри run, не блокирует)
    let mixer = Arc::new(mixer);
    let _mixer_handle = mixer.clone().run();

    // Прописываем на вход line выход сплиттера
    // а на выход - микшер
    let mut line = TranslationLine::new(
        provider,
        output_for_translate,
        Box::new(to_mixer_sender),
        stats_direct,
    )
    .await?;
    // Запускаем фоновую рассылку аудио по выходам сплиттера
    splitter.start();

    let remote_format = AudioFormat::from(line.provider().audio_format());

    // Input DSP
    {
        line.add_input_processor(Processor::Denoiser(Denoiser::new(remote_format.spec())))?;

        line.add_input_processor(Processor::Compressor(Compressor::new(
            NATURAL_VOICE.clone(),
        )))?;

        line.add_input_processor(Processor::IndicatorDiag(Indicator::new(
            direct_input_indicator_tx,
        )))?;
    }

    // Output DSP
    {
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
    let provider_back = OpenAITranslationProvider::new(config_back);

    let stats_back = Arc::new(transono::audio::LatencyStats::default());

    let (back_input_indicator_tx, back_input_indicator_rx) = mpsc::channel(8);
    let (back_output_indicator_tx, back_output_indicator_rx) = mpsc::channel(8);

    let from_speaker_virt = AudioInputCpal::new(from_speaker, stats_back.clone())?;
    let output_hw = AudioOutputCpal::new(playback, stats_back.clone())?;

    // Сплиттер на виртуальном динамике (оригинал собеседника)
    let mut splitter_back =
        AudioSplitter::new(from_speaker_virt.format(), 32, Box::new(from_speaker_virt));
    let translated_out = splitter_back.create_output();
    // Второй выход сплиттера — оригинальный голос собеседника для мониторинга
    let mut original_back_out = splitter_back.create_output();

    // Микшер: перевод (1.0) + оригинал собеседника приглушённо (0.5)
    let mixer_back = AudioMixer::new(translated_out.format());
    let (to_mixer_back_sender, mut to_mixer_back_receiver) =
        AudioLink::new_ports(translated_out.format(), 32);
    let back_translate_ch = mixer_back.add_input(&mut to_mixer_back_receiver, 1.0)?;
    let back_original_ch = mixer_back.add_input(original_back_out.as_mut(), 0.5)?;
    // Выход микшера как отдельный порт, соединяем с реальным динамиком
    let mixer_back_out = mixer_back.get_output();
    let _link_back = AudioLink::new_link(
        translated_out.format(),
        32,
        Box::new(mixer_back_out),
        Box::new(output_hw),
    );
    // Запускаем фоновый цикл микшера (spawn внутри run, не блокирует)
    let mixer_back = Arc::new(mixer_back);
    let _mixer_back_handle = mixer_back.clone().run();

    // TranslationLine "en" -> "ru"
    let mut line_back = TranslationLine::new(
        provider_back,
        translated_out,
        Box::new(to_mixer_back_sender),
        stats_back,
    )
    .await?;
    // Запускаем фоновую рассылку аудио по выходам сплиттера
    splitter_back.start();

    // Input DSP
    {
        line_back.add_input_processor(Processor::IndicatorDiag(Indicator::new(
            back_input_indicator_tx,
        )))?;
    }

    // Output DSP
    {
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
        mixer.clone(),
        direct_translate_ch,
        direct_original_ch,
        mixer_back.clone(),
        back_translate_ch,
        back_original_ch,
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
    let device_id = DeviceId::from_str(format!("{}:{}", host.id(), name).as_str())?;
    println!("Output DeviceId: {}", device_id);
    let device = host.device_by_id(&device_id);
    if let Some(device) = device {
        return Ok(device);
    }
    Err(missing_virtual_device("output", name, language))
}

fn find_virtual_input(host: &Host, name: &str, language: &str) -> Result<Device> {
    let device_id = DeviceId::from_str(format!("{}:{}", host.id(), name).as_str())?;
    let device = host.device_by_id(&device_id);
    if let Some(device) = device {
        return Ok(device);
    }
    Err(missing_virtual_device("input", name, language))
}

fn missing_virtual_device(direction: &str, name: &str, language: &str) -> anyhow::Error {
    anyhow!("virtual {direction} device '{name}' not found; run `transonovirt {language}` first")
}

fn print_latency_stats(snapshot: transono::audio::LatencySnapshot) {
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

fn print_metric(name: &str, m: transono::audio::MetricSnapshot) {
    println!(
        "{} | {:6.2} | {:6.2} | {:6.2} | {:6.2} |",
        name, m.min_ms, m.avg_ms, m.max_ms, m.last_ms
    );
}
