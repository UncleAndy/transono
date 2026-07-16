use std::{
    env,
    thread,
    time::Duration,
};

use anyhow::{bail, Result};
use hound::{SampleFormat, WavReader};

use libereco::audio::{AudioBuffer, AudioFormat, Endianness, PcmFormat, FRAME_CAPACITY};
use libereco::audio::pipewire::PipeWireWorker;

fn main() -> Result<()> {
    let filename = env::args()
        .nth(1)
        .unwrap_or_else(|| "sample.wav".to_string());

    println!("Opening {filename}");

    let mut wav = WavReader::open(&filename)?;

    let spec = wav.spec();

    println!(
        "{} Hz, {} channels, {} bits, {:?}",
        spec.sample_rate,
        spec.channels,
        spec.bits_per_sample,
        spec.sample_format,
    );

    let format = AudioFormat {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        sample_format: PcmFormat::F32(Endianness::Little),
    };

    let (mut producer, consumer) = AudioBuffer::new(32)?;

    let _worker = PipeWireWorker::spawn_output(
        consumer,
        format,
        "wav-smoke".into(),
    )?;

    match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => {
            let mut frame = Vec::<f32>::with_capacity(FRAME_CAPACITY);

            for sample in wav.samples::<f32>() {
                frame.push(sample?);

                if frame.len() == FRAME_CAPACITY {
                    while !producer.send(&frame)? {
                        std::thread::yield_now();
                    }

                    frame.clear();
                }
            }

            if !frame.is_empty() {
                while !producer.send(&frame)? {
                    std::thread::yield_now();
                }
            }
        }

        (SampleFormat::Int, 16) => {
            let mut frame = Vec::<f32>::with_capacity(FRAME_CAPACITY);

            for sample in wav.samples::<i16>() {
                frame.push(sample? as f32 / 32768.0);

                if frame.len() == FRAME_CAPACITY {
                    while !producer.send(&frame)? {
                        std::thread::yield_now();
                    }

                    frame.clear();
                }
            }

            if !frame.is_empty() {
                while !producer.send(&frame)? {
                    std::thread::yield_now();
                }
            }
        }

        _ => bail!("Unsupported WAV format"),
    }

    println!("Waiting for playback...");

    while !producer.is_empty() {
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
