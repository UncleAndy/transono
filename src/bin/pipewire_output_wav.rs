use anyhow::{Result, bail};
use futures_util::SinkExt;
use hound::{SampleFormat, WavReader};
use std::env;
use std::time::Duration;
use tokio::time::sleep;
use libereco::audio::{
    Audio, AudioFormat, AudioOutput, Endianness, FRAME_CAPACITY, PcmAudio, PcmFormat,
    PipeWireOutput,
};
use libereco::audio::AudioCodec::Pcm;

#[tokio::main]
async fn main() -> Result<()> {
    let filename = env::args()
        .nth(1)
        .unwrap_or_else(|| "sample.wav".to_string());

    println!("Opening {filename}");

    let mut wav = WavReader::open(&filename)?;

    let spec = wav.spec();

    println!(
        "{} Hz, {} channels, {} bits, {:?}",
        spec.sample_rate, spec.channels, spec.bits_per_sample, spec.sample_format,
    );

    let format = AudioFormat {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        sample_format: PcmFormat::F32(Endianness::Little),
    };

    let mut output = PipeWireOutput::new(format, "wav-smoke".into(), 0);

    let mut sink = output.sink()?;

    let mut pcm = PcmAudio::new(format.spec(), FRAME_CAPACITY / format.channels as usize);

    match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => {
            let channels = format.channels as usize;
            let mut frame = 0;
            let mut channel = 0;

            for sample in wav.samples::<f32>() {
                pcm.channel_mut(channel)[frame] = sample?;

                channel += 1;
                if channel == channels {
                    channel = 0;
                    frame += 1;
                }

                if frame == pcm.frames() {
                    sink.send(Audio::from_pcm(&pcm)?).await?;

                    frame = 0;
                    channel = 0;
                }
            }
        }
        (SampleFormat::Int, 16) => {
            let channels = format.channels as usize;
            let mut frame = 0;
            let mut channel = 0;

            for sample in wav.samples::<i16>() {
                pcm.channel_mut(channel)[frame] = sample? as f32 / 32768.0;

                channel += 1;
                if channel == channels {
                    channel = 0;
                    frame += 1;
                }

                if frame == pcm.frames() {
                    sink.send(Audio::from_pcm(&pcm)?).await?;

                    frame = 0;
                    channel = 0;
                }
            }
        }

        _ => bail!("Unsupported WAV format"),
    }

    sink.flush().await?;

    sleep(Duration::from_millis(2000)).await;

    output.stop()?;

    Ok(())
}
