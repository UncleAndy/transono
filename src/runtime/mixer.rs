use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::audio::{Audio, AudioFormat, AudioInput, AudioOutput, PcmAudio};
use crate::core::error::{CoreError, Result};
use anyhow::anyhow;

type ChannelId = usize;

struct MixerChannel {
    weight: f32,
    receiver: Receiver<Audio>,
    buffer: VecDeque<f32>,
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
        });

        Ok(id)
    }

    /// The core mixing loop. This should be run in a separate task.
    pub async fn run(self: Arc<Self>) {
        let format = self.format.clone();
        let channels_lock = self.channels.clone();
        let output_tx = self.output_tx.clone();

        loop {
            let result = {
                let mut channels = channels_lock.lock().unwrap();
                
                // 1. Ingest data from all channels
                for channel in channels.values_mut() {
                    while let Ok(audio) = channel.receiver.try_recv() {
                        if let Ok(pcm) = audio.to_pcm() {
                            channel.buffer.extend(pcm.data);
                        }
                    }
                }

                // Determine window size (e.g., 480 frames for 10ms @ 48kHz)
                let frame_size = 480; 
                let sample_count = frame_size * format.channels as usize;

                let has_data = channels.values().any(|c| !c.buffer.is_empty());

                if has_data {
                    let mut mixed_data = vec![0.0f32; sample_count];

                    for channel in channels.values_mut() {
                        let weight = channel.weight;
                        for i in 0..sample_count {
                            if let Some(sample) = channel.buffer.pop_front() {
                                mixed_data[i] += sample * weight;
                            }
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
                    pcm.data = mixed_data;

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
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
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
