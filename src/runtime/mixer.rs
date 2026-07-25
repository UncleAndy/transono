use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use futures_util::stream::BoxStream;
use futures_util::{StreamExt, FutureExt};
use futures_util::SinkExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;
use crate::audio::output::BoxSink;
use crate::audio::{Audio, AudioFormat, AudioInput, AudioOutput, PcmAudio};
use crate::core::error::{CoreError, Result, TransportError};

type ChannelId = usize;

struct MixerChannel {
    weight: f32,
    stream: BoxStream<'static, Audio>,
    buffer: VecDeque<f32>,
    last_timestamp: Option<Instant>,
}

/// Real-time audio mixer.
///
/// Combines multiple input audio streams into a single output stream.
/// Supports weighted mixing and automatic sample rate/channel verification.
pub struct AudioMixer {
    format: AudioFormat,
    channels: Arc<Mutex<HashMap<ChannelId, MixerChannel>>>,
    output_tx: Sender<Audio>,
    output_rx: Mutex<Option<Receiver<Audio>>>,
    next_channel_id: Mutex<ChannelId>,
}

impl AudioMixer {
    /// Creates a new mixer with the specified audio format.
    ///
    /// # Arguments
    ///
    /// * `format` - The target [`AudioFormat`] for the mixed output. All inputs must match this format.
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

    /// Adds a new input stream to the mixer.
    ///
    /// # Arguments
    ///
    /// * `input` - A mutable reference to an object implementing [`AudioInput`].
    /// * `weight` - Volume multiplier for this channel (usually 0.0 to 1.0).
    ///
    /// # Returns
    ///
    /// Returns a [`Result`] containing a unique [`ChannelId`] if successful.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] (with incompatible format message) if the input format
    /// does not match the mixer's format, or if getting the input stream fails.
    pub fn add_input(&self, input: &mut dyn AudioInput, weight: f32) -> Result<ChannelId> {
        if input.format() != self.format {
            return Err(CoreError::Internal(format!(
                "Incompatible audio format: expected {:?}, got {:?}",
                self.format,
                input.format()
            )));
        }

        let input_stream = input.stream()?;
        let mut channels = self.channels.lock().unwrap();
        let mut id_gen = self.next_channel_id.lock().unwrap();
        
        let id = *id_gen;
        *id_gen += 1;

        channels.insert(id, MixerChannel {
            weight,
            stream: input_stream,
            buffer: VecDeque::new(),
            last_timestamp: None,
        });

        Ok(id)
    }

    /// Runs the mixer's processing loop.
    ///
    /// This should be spawned in a separate task. It continuously pulls
    /// data from all inputs, mixes it, and pushes to the output.
    pub async fn run(self: Arc<Self>) {
        let format = self.format.clone();
        let channels_lock = self.channels.clone();
        let output_tx = self.output_tx.clone();

        let frame_ms = 10;
        let frame_size = (format.sample_rate as u64 * frame_ms / 1000) as usize;
        let sample_count = frame_size * format.channels as usize;
        let mut mixed_data = vec![0.0f32; sample_count];
        let sample_duration = Duration::from_nanos(1_000_000_000 / format.sample_rate as u64);
        let max_buffer_samples = format.sample_rate as usize * format.channels as usize;

        loop {
            let result = {
                let mut channels = channels_lock.lock().unwrap();

                for channel in channels.values_mut() {
                    while let Some(audio) = channel.stream.next().now_or_never().flatten() {
                        if let Ok(pcm) = audio.to_pcm() {
                            if channel.buffer.is_empty() {
                                channel.last_timestamp = Some(audio.capture_timestamp());
                            }
                            channel.buffer.extend(pcm.data);

                            if channel.buffer.len() > max_buffer_samples {
                                let to_remove = channel.buffer.len() - max_buffer_samples;
                                channel.buffer.drain(0..to_remove);
                            }
                        }
                    }
                }

                mixed_data.fill(0.0);
                let mut has_data = false;
                let mut first_ts = None;

                for channel in channels.values_mut() {
                    if channel.buffer.len() >= sample_count {
                        has_data = true;
                        if first_ts.is_none() {
                            first_ts = channel.last_timestamp;
                        }

                        for i in 0..sample_count {
                            mixed_data[i] += channel.buffer.pop_front().unwrap() * channel.weight;
                        }
                        
                        if let Some(ref mut ts) = channel.last_timestamp {
                            *ts += sample_duration * (frame_size as u32);
                        }
                    }
                }

                if has_data {
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
                    pcm.data = mixed_data.clone();
                    
                    let mut audio = Audio::from_pcm(&pcm).unwrap();
                    if let Some(ts) = first_ts {
                        audio.set_capture_timestamp(ts);
                    }
                    Some(audio)
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

impl AudioOutput for AudioMixer {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
        Ok(Box::pin(PollSender::new(self.output_tx.clone())
            .sink_map_err(|_| CoreError::Transport(TransportError::ConnectionClosed))))
    }

    fn start(&mut self) -> Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}

impl AudioInput for AudioMixer {
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        let mut rx_lock = self.output_rx.lock().unwrap();
        let receiver = rx_lock.take().ok_or_else(|| CoreError::Internal("Mixer receiver already taken".to_string()))?;
        Ok(ReceiverStream::new(receiver).boxed())
    }

    fn start(&self) -> Result<()> {
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, AudioFormat, PcmAudio, AudioInput};
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
        fn stream(&mut self) -> crate::core::error::Result<BoxStream<'static, Audio>> {
            let receiver = self.receiver.take().ok_or_else(|| crate::core::error::CoreError::Internal("receiver taken".to_string()))?;
            Ok(ReceiverStream::new(receiver).boxed())
        }
        fn start(&self) -> crate::core::error::Result<()> { Ok(()) }
        fn stop(&self) -> crate::core::error::Result<()> { Ok(()) }
        fn format(&self) -> AudioFormat { self.format.clone() }
    }

    #[tokio::test]
    async fn test_mixer_format_compatibility() {
        let format = create_test_format();
        let mixer = AudioMixer::new(format.clone());

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
        let mut mixer = AudioMixer::new(format.clone());
        let mut stream_out = mixer.stream().unwrap();
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

        let mixed_audio = stream_out.next().await.expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();

        for &sample in pcm.data.iter() {
            assert!((sample - 0.6).abs() < 1e-6, "Expected 0.6, got {}", sample);
        }
    }

    #[tokio::test]
    async fn test_clamping() {
        let format = create_test_format();
        let mut mixer = AudioMixer::new(format.clone());
        let mut stream_out = mixer.stream().unwrap();
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

        let mixed_audio = stream_out.next().await.expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();

        for &sample in pcm.data.iter() {
            assert_eq!(sample, 1.0, "Expected clamped 1.0, got {}", sample);
        }
    }
}
