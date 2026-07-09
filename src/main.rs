use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio::signal;
use realtime_translator::audio::{AudioDevicesCpal, AudioInput, AudioInputCpal, AudioOutput, AudioOutputCpal, Processor};
use realtime_translator::audio::processors::channel_converter::ChannelConverter;
use realtime_translator::audio::processors::resampler::Resampler;
use realtime_translator::providers::openai::translation::{
    OpenAITranslationConfig,
    OpenAITranslationProvider,
};
use std::process::Command;
use std::time::Duration;
use realtime_translator::runtime::TranslationLine;

/*
    Создания устройства воспроизведения (Виртуального динамика):
    $ pactl load-module module-null-sink
        sink_name=translator_out
        sink_properties=device.description="Translator.EN.Speaker"

    Создание виртуального микрофона:
    $ pactl load-module module-remap-source
        master=translator_out.monitor
        source_name=translator_in
        source_properties=device.description="Translator.EN.Microphone"

    Оба метода возвращают числовые идентификаторы, которые можно использовать для их удаления:
    $ pactl unload-module <num id>
 */


#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls provider");

    let devices = AudioDevicesCpal::new();

    let capture = devices.default_input()?;
    let playback = devices.default_output()?;


    let virtual_devices = VirtualDevices::create("EN");

    tokio::time::sleep(Duration::from_secs(1)).await;

    let host = devices.host();

    let virtual_speaker = host
        .output_devices()?
        .find(|d| {
            d.description().unwrap().name().contains("translator_en_speaker")
        })
        .expect("virtual speaker not found");

    let virtual_microphone = host
        .input_devices()?
        .find(|d| {
            d.description().unwrap().name().contains("translator_en_microphone")
        })
        .expect("virtual microphone not found");

    println!("Translator Example");
    println!("===========================");
    println!();

    println!("Input device : {}", capture.description()?);
    println!("Input format: {:?}", capture.default_input_config()?.sample_format());
    println!("Output device: {}", playback.description()?);
    println!("Output format: {:?}", playback.default_output_config()?.sample_format());

    let config = OpenAITranslationConfig::from_env()?
        .with_lang("en")
        .clone();

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


pub struct VirtualDevices {
    pub sink_name: String,
    pub source_name: String,

    sink_module: u32,
    source_module: u32,
}

impl VirtualDevices {
    pub fn create(lang: &str) -> Result<Self> {
        let lang = lang.to_lowercase();

        let sink_name = format!("translator_{lang}_speaker");
        let source_name = format!("translator_{lang}_microphone");

        //
        // Virtual speaker
        //
        let sink_module = load_module(
            "module-null-sink",
            &[
                ("sink_name", &sink_name),
                (
                    "sink_properties",
                    &format!(
                        "device.description=Translator.{}.Speaker",
                        lang.to_uppercase()
                    ),
                ),
            ],
        )?;

        //
        // Virtual microphone
        //
        let source_module = load_module(
            "module-remap-source",
            &[
                ("master", &format!("{sink_name}.monitor")),
                ("source_name", &source_name),
                (
                    "source_properties",
                    &format!(
                        "device.description=Translator.{}.Microphone",
                        lang.to_uppercase()
                    ),
                ),
            ],
        )?;

        Ok(Self {
            sink_name,
            source_name,
            sink_module,
            source_module,
        })
    }
}

impl Drop for VirtualDevices {
    fn drop(&mut self) {
        let _ = unload_module(self.source_module);
        let _ = unload_module(self.sink_module);
    }
}

fn load_module(
    module: &str,
    args: &[(&str, &str)],
) -> Result<u32> {
    let mut cmd = Command::new("pactl");

    cmd.arg("load-module");
    cmd.arg(module);

    for (k, v) in args {
        cmd.arg(format!("{k}={v}"));
    }

    let out = cmd.output()?;

    if !out.status.success() {
        return Err(anyhow!(
            "{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    Ok(String::from_utf8(out.stdout)?
        .trim()
        .parse()?)
}

fn unload_module(id: u32) -> Result<()> {
    let status = Command::new("pactl")
        .args(["unload-module", &id.to_string()])
        .status()?;

    if !status.success() {
        return Err(anyhow!("pactl unload-module failed"));
    }

    Ok(())
}
