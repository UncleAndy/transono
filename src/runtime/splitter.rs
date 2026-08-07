use tokio_util::sync::CancellationToken;
use futures_util::StreamExt;
use crate::audio::{AudioFormat, AudioInput};
use crate::runtime::AudioLink;
use crate::runtime::receiver_port::ReceiverPort;
use crate::runtime::sender_port::SenderPort;

#[allow(unused)]
/// A component for splitting one audio stream into multiple outputs.
///
/// Clones incoming audio frames and broadcasts them to all registered outputs.
pub struct AudioSplitter {
    cancel: CancellationToken,

    input: Box<dyn AudioInput>,
    format: AudioFormat,
    capacity: usize,

    outputs: Vec<SenderPort>,
}

impl AudioSplitter {
    #[allow(unused)]
    /// Creates a new audio splitter with the specified format and capacity.
    ///
    /// # Arguments
    ///
    /// * `format` - The [`AudioFormat`] for all input and output streams.
    /// * `capacity` - The buffer capacity for each output channel.
    pub fn new(format: AudioFormat, capacity: usize, input: Box<dyn AudioInput>) -> Self {
        Self {
            input,
            cancel: CancellationToken::new(),
            outputs: Vec::new(),
            format,
            capacity,
        }
    }

    #[allow(unused)]
    /// Creates a new output port and returns a reference to it.
    ///
    /// # Returns
    ///
    /// Returns a reference to the newly created [`ReceiverPort`].
    pub fn create_output(&mut self) -> Box<ReceiverPort> {
        let (link_sender_port, link_receiver_port) =
            AudioLink::new_ports(self.format, self.capacity);

        self.outputs.push(link_sender_port);

        Box::new(link_receiver_port)
    }

    #[allow(unused)]
    /// Starts the splitter's processing loop in a background task.
    pub fn start(&mut self) {
        let cancel = self.cancel.clone();
        let senders: Vec<_> = self.outputs.iter().map(|o| o.sender()).collect();

        let stream_result = self.input.stream();
        let start_result = self.input.start();

        if let (Ok(mut stream), Ok(())) = (stream_result, start_result) {
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = cancel.cancelled() => {
                            break;
                        }
                        opt_audio = stream.next() => {
                            let Some(audio) = opt_audio else {
                                break;
                            };
                            for tx in &senders {
                                let _ = tx.send(audio.clone()).await;
                            }
                        }
                    }
                }
            });
        }
    }

    /// Stops the splitter's processing loop.
    pub fn stop(&self) {
        self.cancel.cancel();
    }
}

impl Drop for AudioSplitter {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, EncodedAudioFormat, PcmAudio};
    use symphonia::core::audio::AudioSpec;

    /// Build an `Audio` chunk from a planar `f32` sample vector.
    fn make_audio(format: &AudioFormat, samples: Vec<f32>) -> Audio {
        let frames = samples.len() / format.channels as usize;
        let mut pcm = PcmAudio::new(
            AudioSpec::new(format.sample_rate, symphonia::core::audio::Channels::Discrete(format.channels)),
            frames,
        );
        pcm.data = samples;
        Audio::from_pcm(&pcm).expect("make_audio")
    }

    #[tokio::test]
    async fn test_audio_splitter() {
        let format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let (tx_port, rx_port) = AudioLink::new_ports(format.clone(), 10);

        let mut splitter = AudioSplitter::new(format.clone(), 10, Box::new(rx_port));
        let mut out1 = splitter.create_output();
        let mut out2 = splitter.create_output();

        splitter.start();

        // Non-trivial stereo content: channel0 = 0.3, channel1 = 0.7 (planar)
        let frames = 10usize;
        let ch = format.channels as usize;
        let mut data = vec![0.0f32; frames * ch];
        for i in 0..frames {
            data[i] = 0.3; // channel 0
            if ch > 1 {
                data[frames + i] = 0.7; // channel 1
            }
        }
        let pcm = PcmAudio::new(
            AudioSpec::new(format.sample_rate, symphonia::core::audio::Channels::Discrete(format.channels)),
            frames,
        );
        let _ = pcm; // pcm.data is private; build via make_audio instead
        let audio = make_audio(&format, data.clone());

        tx_port.sender().send(audio.clone()).await.unwrap();

        let mut stream1 = out1.stream().unwrap();
        let mut stream2 = out2.stream().unwrap();

        let received1 = tokio::time::timeout(std::time::Duration::from_secs(1), stream1.next()).await;
        let received2 = tokio::time::timeout(std::time::Duration::from_secs(1), stream2.next()).await;

        assert!(received1.is_ok());
        assert!(received2.is_ok());
        let a1 = received1.unwrap().unwrap();
        let a2 = received2.unwrap().unwrap();

        // Both outputs must carry identical PCM data.
        let p1 = a1.to_pcm().unwrap();
        let p2 = a2.to_pcm().unwrap();
        assert_eq!(p1.data.len(), data.len(), "splitter changed frame size");
        for (x, y) in p1.data.iter().zip(p2.data.iter()) {
            assert!((x - y).abs() < 1e-6, "splitter outputs diverged: {x} vs {y}");
        }
        // And must match what we sent.
        for (x, y) in p1.data.iter().zip(data.iter()) {
            assert!((x - y).abs() < 1e-6, "splitter altered sample: {x} vs {y}");
        }

        splitter.stop();
    }
}
