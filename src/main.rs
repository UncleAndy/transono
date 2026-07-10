use anyhow::{Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait};
use std::process::Command;
use std::time::Duration;
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio::signal;

use realtime_translator::audio::processors::channel_converter::ChannelConverter;
use realtime_translator::audio::processors::resampler::Resampler;
use realtime_translator::audio::{
    AudioDevicesCpal, AudioInput, AudioInputCpal, AudioOutput, AudioOutputCpal, Processor,
};
use realtime_translator::providers::openai::translation::{
    OpenAITranslationConfig, OpenAITranslationProvider,
};
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

    let input_hw = AudioInputCpal::new(capture)?;
    let to_microphone_virt = AudioOutputCpal::new(to_microphone)?;

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
        TranslationLine::new(provider, Box::new(input_hw), Box::new(to_microphone_virt)).await?;

    // Input DSP
    {
        line.add_input_processor(Processor::Dsp(Box::new(ChannelConverter::new(
            mono.clone(),
        ))))?;

        line.add_input_processor(Processor::Dsp(Box::new(Resampler::new(
            AudioSpec::new(input_sample_rate, mono.clone()),
            remote_spec.rate(),
        )?)))?;
    }

    // Output DSP
    {
        line.add_output_processor(Processor::Dsp(Box::new(Resampler::new(
            AudioSpec::new(remote_spec.rate(), mono.clone()),
            output_sample_rate,
        )?)))?;

        line.add_output_processor(Processor::Dsp(Box::new(ChannelConverter::new(
            stereo.clone(),
        ))))?;
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

    let from_speaker_virt = AudioInputCpal::new(from_speaker)?;
    let output_hw = AudioOutputCpal::new(playback)?;

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
    )
    .await?;

    // Input DSP
    {
        line_back.add_input_processor(Processor::Dsp(Box::new(ChannelConverter::new(
            mono.clone(),
        ))))?;

        line_back.add_input_processor(Processor::Dsp(Box::new(Resampler::new(
            AudioSpec::new(input_back_sample_rate, mono.clone()),
            remote_back_spec.rate(),
        )?)))?;
    }

    // Output DSP
    {
        line_back.add_output_processor(Processor::Dsp(Box::new(Resampler::new(
            AudioSpec::new(remote_back_spec.rate(), mono.clone()),
            output_back_sample_rate,
        )?)))?;

        line_back.add_output_processor(Processor::Dsp(Box::new(ChannelConverter::new(
            stereo.clone(),
        ))))?;
    }

    /*
    ----------------------------------------------------------------------------------
     */

    println!("Run lines...");
    line.run().await?;
    line_back.run().await?;

    println!("Можете говорить.");
    println!("Press Ctrl+C to stop.");

    signal::ctrl_c().await?;

    println!("Stopping...");

    println!("Stop back line...");
    line_back.stop().await?;
    println!("Stop direct line...");
    line.stop().await?;

    println!("Remove virtual devices...");
    drop(virtual_devices);

    println!("Done.");

    Ok(())
}

#[derive(Clone)]
pub struct VirtualDevices {
    pub from: VirtualDevicePair,
    pub to: VirtualDevicePair,
}

#[derive(Clone)]
pub struct VirtualDevicePair {
    pub sink_name: String,
    pub source_name: String,

    pub output_name: String,
    pub input_name: String,

    sink_module: u32,
    source_module: u32,
}

impl VirtualDevices {
    /// Возвращает набор:
    /// - Объект VirtualDevices для сохранения его времени жизни
    /// - Строку с именем устройства воспроизведения для передачи аудио в микрофон
    /// - Строку с именем устройства чтения для чтения аудиоданных со встречи
    pub fn create(lang: &str) -> Result<(Self, String, String)> {
        let to_pair = create_pair(lang, "ToMeeting")?;
        let from_pair = create_pair(lang, "FromMeeting")?;
        Ok((
            Self {
                to: to_pair.clone(),
                from: from_pair.clone(),
            },
            to_pair.output_name,
            from_pair.input_name,
        ))
    }
}

fn create_pair(lang: &str, prefix: &str) -> Result<VirtualDevicePair> {
    let lang = lang.to_lowercase();

    let sink_name = format!("translator_{lang}_{prefix}_speaker");
    let source_name = format!("translator_{lang}_{prefix}_microphone");

    let output_name = format!("Translator.{}.{}.Speaker", lang.to_uppercase(), prefix,);

    let input_name = format!("Translator.{}.{}.Microphone", lang.to_uppercase(), prefix,);

    let sink_module = load_module(
        "module-null-sink",
        &[
            ("sink_name", &sink_name),
            (
                "sink_properties",
                &format!("device.description={output_name}"),
            ),
        ],
    )?;

    let source_module = load_module(
        "module-remap-source",
        &[
            ("master", &format!("{sink_name}.monitor")),
            ("source_name", &source_name),
            (
                "source_properties",
                &format!("device.description={input_name}"),
            ),
        ],
    )?;

    Ok(VirtualDevicePair {
        sink_name,
        source_name,
        output_name,
        input_name,
        sink_module,
        source_module,
    })
}

impl Drop for VirtualDevices {
    fn drop(&mut self) {
        println!("Unload modules for virtual devices...");
        let _ = unload_module(self.from.source_module);
        let _ = unload_module(self.from.sink_module);
        let _ = unload_module(self.to.source_module);
        let _ = unload_module(self.to.sink_module);
        println!("Unload modules done.");
    }
}

fn load_module(module: &str, args: &[(&str, &str)]) -> Result<u32> {
    let mut cmd = Command::new("pactl");

    cmd.arg("load-module");
    cmd.arg(module);

    for (k, v) in args {
        cmd.arg(format!("{k}={v}"));
    }

    let out = cmd.output()?;

    if !out.status.success() {
        return Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr)));
    }

    Ok(String::from_utf8(out.stdout)?.trim().parse()?)
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
