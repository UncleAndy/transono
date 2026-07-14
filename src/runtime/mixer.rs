use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audio::{Audio, AudioFormat, AudioInput, AudioOutput, PcmAudio};
use crate::core::error::{CoreError, Result};
use anyhow::anyhow;

type ChannelId = usize;

struct MixerChannel {
    weight: f32,
    receiver: Receiver<Audio>,
    buffer: VecDeque<f32>,
    last_timestamp: Option<Instant>,
}

pub struct Mixer {
    format: AudioFormat,
    channels: Arc<Mutex<HashMap<ChannelId, MixerChannel>>>,
    output_tx: Sender<Audio>,
    output_rx: Mutex<Option<Receiver<Audio>>>,
    next_channel_id: Mutex<ChannelId>,
}

impl Mixer {
    pub fn new(format: AudioFormat) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            format,
            channels: Arc::new(Mutex::new(HashMap::new())),
            output_tx: tx,
            output_rx: Mutex::new(Some(rx)),
            next_channel_id: Mutex::new(0),
        }
    }

    /// Adds an input source to the mixer.
    /// The input must implement `AudioInput` so we can take its receiver.
    pub fn add_input(&self, input: &mut dyn AudioInput, weight: f32) -> Result<ChannelId> {
        if input.format() != &self.format {
            return Err(CoreError::Other(anyhow!(
                "Incompatible audio format: expected {:?}, got {:?}",
                self.format,
                input.format()
            )));
        }

        let receiver = input.take_receiver()?;
        let mut channels = self.channels.lock().unwrap();
        let mut id_gen = self.next_channel_id.lock().unwrap();
        
        let id = *id_gen;
        *id_gen += 1;

        channels.insert(id, MixerChannel {
            weight,
            receiver,
            buffer: VecDeque::new(),
            last_timestamp: None,
        });

        Ok(id)
    }

    pub async fn run(self: Arc<Self>) {
        let format = self.format.clone();
        let channels_lock = self.channels.clone();
        let output_tx = self.output_tx.clone();

        // 10ms frame size
        let frame_ms = 10;
        let frame_size = (format.sample_rate as u64 * frame_ms / 1000) as usize;
        let sample_count = frame_size * format.channels as usize;
        let mut mixed_data = vec![0.0f32; sample_count];
        let sample_duration = Duration::from_nanos(1_000_000_000 / format.sample_rate as u64);
        let max_buffer_samples = format.sample_rate as usize * format.channels as usize; // 1 second buffer limit

        loop {
            let result = {
                let mut channels = channels_lock.lock().unwrap();

                // 1. Ingest data from all channels
                for channel in channels.values_mut() {
                    while let Ok(audio) = channel.receiver.try_recv() {
                        if let Ok(pcm) = audio.to_pcm() {
                            if channel.buffer.is_empty() {
                                channel.last_timestamp = Some(audio.capture_timestamp());
                            }
                            channel.buffer.extend(pcm.data);

                            // Limit buffer size to prevent memory leaks/excessive latency
                            if channel.buffer.len() > max_buffer_samples {
                                let to_remove = channel.buffer.len() - max_buffer_samples;
                                channel.buffer.drain(0..to_remove);
                                if let Some(ts) = &mut channel.last_timestamp {
                                    let frames_removed = to_remove / format.channels as usize;
                                    *ts += sample_duration * frames_removed as u32;
                                }
                            }
                        }
                    }
                }

                let has_data = channels.values().any(|c| !c.buffer.is_empty());

                if has_data {
                    mixed_data.fill(0.0);
                    let mut min_ts: Option<Instant> = None;

                    for channel in channels.values_mut() {
                        if channel.buffer.is_empty() {
                            continue;
                        }

                        if let Some(ts) = channel.last_timestamp {
                            min_ts = Some(min_ts.map_or(ts, |m| m.min(ts)));
                        }

                        let weight = channel.weight;
                        let count = sample_count.min(channel.buffer.len());

                        // Use slices for better performance and potential SIMD auto-vectorization
                        let (s1, s2) = channel.buffer.as_slices();

                        let part1 = count.min(s1.len());
                        for i in 0..part1 {
                            mixed_data[i] += s1[i] * weight;
                        }

                        let part2 = count.saturating_sub(s1.len());
                        for i in 0..part2 {
                            mixed_data[part1 + i] += s2[i] * weight;
                        }

                        channel.buffer.drain(0..count);
                        if let Some(ts) = &mut channel.last_timestamp {
                            let frames_consumed = count / format.channels as usize;
                            *ts += sample_duration * frames_consumed as u32;
                        }
                    }

                    // Avoid overload: Clamp to [-1.0, 1.0]
                    for sample in mixed_data.iter_mut() {
                        *sample = sample.clamp(-1.0, 1.0);
                    }

                    let mut pcm = PcmAudio::new(
                        symphonia::core::audio::AudioSpec::new(
                            format.sample_rate,
                            symphonia::core::audio::Channels::Discrete(format.channels),
                        ),
                        frame_size,
                    );
                    pcm.data.copy_from_slice(&mixed_data);
                    if let Some(ts) = min_ts {
                        pcm.capture_timestamp = ts;
                    }

                    if let Ok(audio) = Audio::from_pcm(&pcm) {
                        Some(audio)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(audio) = result {
                let _ = output_tx.send(audio).await;
            } else {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }
    }
}

impl AudioOutput for Mixer {
    fn clone_sender(&mut self) -> Result<Sender<Audio>> {
        // A Mixer doesn't typically act as a simple AudioOutput destination 
        // for a single stream via a Sender, because it manages multiple 
        // named channels. However, for trait compliance, we could return the output_tx
        // or a specialized internal sender. 
        // Since the `add_input` method is the primary way to add channels,
        // we'll provide a basic implementation.
        Ok(self.output_tx.clone())
    }

    fn start(&self) -> Result<()> {
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> &AudioFormat {
        &self.format
    }
}

impl AudioInput for Mixer {
    fn take_receiver(&mut self) -> Result<Receiver<Audio>> {
        let mut rx_lock = self.output_rx.lock().unwrap();
        rx_lock.take().ok_or_else(|| CoreError::Other(anyhow!("Mixer receiver already taken")))
    }

    fn start(&self) -> Result<()> {
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> &AudioFormat {
        &self.format
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use crate::audio::{Audio, AudioFormat, PcmAudio, AudioInput};
    use crate::runtime::Mixer;
    use tokio::sync::mpsc;
    use std::sync::Arc;

    fn create_test_format() -> AudioFormat {
        let internal = crate::audio::EncodedAudioFormat::internal_format();
        let spec = internal.spec();
        AudioFormat {
            sample_rate: spec.rate(),
            channels: spec.channels().count() as u16,
            sample_format: match internal.codec() {
                crate::audio::AudioCodec::Pcm(fmt) => fmt,
                _ => crate::audio::PcmFormat::F32(crate::audio::Endianness::Little),
            },
        }
    }

    fn create_audio_with_samples(format: &AudioFormat, samples: Vec<f32>) -> Audio {
        let frames = samples.len() / format.channels as usize;
        let mut pcm = PcmAudio::new(
            symphonia::core::audio::AudioSpec::new(
                format.sample_rate,
                symphonia::core::audio::Channels::Discrete(format.channels),
            ),
            frames,
        );
        pcm.data = samples;
        Audio::from_pcm(&pcm).expect("Failed to create audio from pcm")
    }

    struct MockInput {
        format: AudioFormat,
        receiver: Option<mpsc::Receiver<Audio>>,
    }

    impl MockInput {
        fn new(format: AudioFormat, rx: mpsc::Receiver<Audio>) -> Self {
            Self { format, receiver: Some(rx) }
        }
    }

    impl AudioInput for MockInput {
        fn take_receiver(&mut self) -> crate::core::error::Result<mpsc::Receiver<Audio>> {
            self.receiver.take().ok_or_else(|| crate::core::error::CoreError::Other(anyhow::anyhow!("receiver taken")))
        }
        fn start(&self) -> crate::core::error::Result<()> { Ok(()) }
        fn stop(&self) -> crate::core::error::Result<()> { Ok(()) }
        fn format(&self) -> &AudioFormat { &self.format }
    }

    #[tokio::test]
    async fn test_mixer_format_compatibility() {
        let format = create_test_format();
        let mixer = Mixer::new(format.clone());

        let correct_format = format.clone();
        let (_tx_in, rx_in) = mpsc::channel(10);
        let mut input_ok = MockInput::new(correct_format, rx_in);

        assert!(mixer.add_input(&mut input_ok, 1.0).is_ok());

        let wrong_format = AudioFormat {
            sample_rate: 44100,
            ..format.clone()
        };
        let (_tx_wrong, rx_wrong) = mpsc::channel(10);
        let mut input_bad = MockInput::new(wrong_format, rx_wrong);

        assert!(mixer.add_input(&mut input_bad, 1.0).is_err());
    }

    #[tokio::test]
    async fn test_mixing_logic() {
        let format = create_test_format();
        let mut mixer = Mixer::new(format.clone());
        let mut rx_out = mixer.take_receiver().unwrap();
        let mixer = Arc::new(mixer);

        // Input 1: all 0.5
        let (tx1, rx1) = mpsc::channel(10);
        let mut input1 = MockInput::new(format.clone(), rx1);
        mixer.add_input(&mut input1, 1.0).unwrap();

        // Input 2: all 0.2
        let (tx2, rx2) = mpsc::channel(10);
        let mut input2 = MockInput::new(format.clone(), rx2);
        mixer.add_input(&mut input2, 0.5).unwrap();

        // Total should be 0.5 * 1.0 + 0.2 * 0.5 = 0.5 + 0.1 = 0.6
        let samples1 = vec![0.5f32; 480 * format.channels as usize];
        let samples2 = vec![0.2f32; 480 * format.channels as usize];

        tx1.send(create_audio_with_samples(&format, samples1)).await.unwrap();
        tx2.send(create_audio_with_samples(&format, samples2)).await.unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move {
            mixer_clone.run().await;
        });

        let mixed_audio = rx_out.recv().await.expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();

        for &sample in pcm.data.iter() {
            assert!((sample - 0.6).abs() < 1e-6, "Expected 0.6, got {}", sample);
        }
    }

    #[tokio::test]
    async fn test_clamping() {
        let format = create_test_format();
        let mut mixer = Mixer::new(format.clone());
        let mut rx_out = mixer.take_receiver().unwrap();
        let mixer = Arc::new(mixer);

        let (tx1, rx1) = mpsc::channel(10);
        let mut input1 = MockInput::new(format.clone(), rx1);
        mixer.add_input(&mut input1, 1.0).unwrap();

        let (tx2, rx2) = mpsc::channel(10);
        let mut input2 = MockInput::new(format.clone(), rx2);
        mixer.add_input(&mut input2, 1.0).unwrap();

        // 0.8 + 0.8 = 1.6 -> should clamp to 1.0
        let samples1 = vec![0.8f32; 480 * format.channels as usize];
        let samples2 = vec![0.8f32; 480 * format.channels as usize];

        tx1.send(create_audio_with_samples(&format, samples1)).await.unwrap();
        tx2.send(create_audio_with_samples(&format, samples2)).await.unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move {
            mixer_clone.run().await;
        });

        let mixed_audio = rx_out.recv().await.expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();

        for &sample in pcm.data.iter() {
            assert_eq!(sample, 1.0, "Expected clamped 1.0, got {}", sample);
        }
    }
}
