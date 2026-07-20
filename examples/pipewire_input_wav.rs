use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;

use transono::audio::{AudioInput, AudioFormat, PipeWireInput, PcmAudio, DspProcessor};
use transono::audio::diagnost::wav_dump::WavDump;
use transono::ctl::create_backend;

#[tokio::main]
async fn main() -> Result<()> {
    let language = "en";

    let backend = create_backend()
        .context("failed to create backend")?;

    backend.init(language)?;

    let virt = backend
        .devices(language)
        .context("failed to resolve virtual devices")?;

    println!(
        "Using virtual input: {}",
        virt.internal_from_meeting_microphone_in
    );

    let node_id = find_node_by_name(
        &virt.internal_from_meeting_microphone_in,
    )?
        .ok_or_else(|| anyhow!("PipeWire node not found"))?;

    //
    // Лучше взять формат устройства из PipeWire.
    // Если сейчас у тебя нет enumerate_nodes(),
    // можно временно оставить 48kHz/F32/mono.
    //
    let format = AudioFormat {
        sample_rate: 48_000,
        channels: 2,
        sample_format: transono::audio::PcmFormat::F32(
            transono::audio::Endianness::Little,
        ),
    };

    let mut input = PipeWireInput::new(
        format,
        virt.internal_from_meeting_microphone_in.clone(),
        node_id,
    )?;

    let mut stream = input.stream()?;

    let mut pcm = PcmAudio::new(format.spec(), 0);

    let mut wav = WavDump::new(
        "capture.wav",
        format.spec(),
    )?;

    println!("Recording...");
    println!("Press Ctrl+C to stop.\n");

    let mut packets = 0usize;

    while let Some(audio) = stream.next().await {
        packets += 1;

        audio.to_pcm_into(&mut pcm)?;

        println!(
            "#{:05} frames={} channels={}",
            packets,
            pcm.frames(),
            pcm.spec.channels().count()
        );

        wav.process(&mut pcm)?;
    }

    Ok(())
}

fn find_node_by_name(name: &str) -> Result<Option<u32>> {
    use transono::audio::{AudioDeviceFactory, AudioDeviceId};
    use transono::audio::pipewire::device::PipeWireDeviceFactory;

    let factory = PipeWireDeviceFactory;
    let devices = factory.enumerate_devices()?;

    for device in devices {
        if device.name == name {
            if let AudioDeviceId::Numeric(id) = device.id {
                return Ok(Some(id as u32));
            }
        }
    }

    Ok(None)
}
