use std::{thread, time::Duration};

use anyhow::Result;

use transono::audio::{AudioBuffer, AudioFormat, Endianness, PcmFormat};
use transono::audio::pipewire::PipeWireWorker;

fn main() -> Result<()> {

    println!("Starting PipeWire echo test...");

    let format = AudioFormat {
        sample_rate: 48_000,
        channels: 2,
        sample_format: PcmFormat::F32(Endianness::Little),
    };

    // Общий буфер между захватом и воспроизведением.
    let (producer, consumer) = AudioBuffer::new(32)?;

    // Захват с микрофона.
    let _capture = PipeWireWorker::spawn_input(
        producer,
        format,
        "echo-input".into(),
        None,
    )?;

    // Вывод в колонки.
    let _playback = PipeWireWorker::spawn_output(
        consumer,
        format,
        "echo-output".into(),
        None,
    )?;

    println!("Echo is running.");
    println!("Speak into the microphone. Press Ctrl+C to exit.");

    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
