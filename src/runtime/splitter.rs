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

    #[tokio::test]
    async fn test_audio_splitter() {
        let format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let (tx_port, rx_port) = AudioLink::new_ports(format.clone(), 10);

        let mut splitter = AudioSplitter::new(format.clone(), 10, Box::new(rx_port));
        let mut out1 = splitter.create_output();
        let mut out2 = splitter.create_output();

        splitter.start();

        let pcm = PcmAudio::new(
            AudioSpec::new(format.sample_rate, symphonia::core::audio::Channels::Discrete(format.channels)),
            10,
        );
        let audio = Audio::from_pcm(&pcm).unwrap();

        tx_port.sender().send(audio.clone()).await.unwrap();

        let mut stream1 = out1.stream().unwrap();
        let mut stream2 = out2.stream().unwrap();

        let received1 = tokio::time::timeout(std::time::Duration::from_secs(1), stream1.next()).await;
        let received2 = tokio::time::timeout(std::time::Duration::from_secs(1), stream2.next()).await;

        assert!(received1.is_ok());
        assert!(received2.is_ok());
        assert!(received1.unwrap().is_some());
        assert!(received2.unwrap().is_some());

        splitter.stop();
    }
}
