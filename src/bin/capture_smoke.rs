use std::{
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use hound::{SampleFormat, WavSpec, WavWriter};

use libereco::audio::{AudioBuffer, AudioFormat, Endianness, PcmFormat};
use libereco::audio::pipewire::PipeWireWorker;

fn main() -> Result<()> {
    let format = AudioFormat {
        sample_rate: 48_000,
        channels: 2,
        sample_format: PcmFormat::F32(Endianness::Little),
    };

    let (producer, mut consumer) = AudioBuffer::new(32)?;

    let _worker = PipeWireWorker::spawn_input(
        producer,
        format,
        "capture-smoke".into(),
    )?;

    println!("Recording for 5 seconds...");

    let spec = WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create("capture.wav", spec)?;

    let started = Instant::now();

    while started.elapsed() < Duration::from_secs(5) {
        let Some(frame) = consumer.receive() else {
            thread::sleep(Duration::from_millis(1));
            continue;
        };

        consumer.read(frame, |audio| {
            for &sample in audio.samples() {
                writer.write_sample(sample).unwrap();
            }
        });

        consumer.release(frame)?;
    }

    writer.finalize()?;

    println!("Saved to capture.wav");

    Ok(())
}
