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
mod mixer_tests;
