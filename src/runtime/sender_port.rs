use tokio::sync::mpsc::Sender;
use tokio_util::sync::PollSender;
use futures_util::SinkExt;
use crate::audio::output::BoxSink;
use crate::audio::{Audio, AudioFormat, AudioOutput};
use crate::core::error::{CoreError, Result, TransportError};

/// An input entry point for an audio graph.
///
/// Implements [`AudioOutput`] to receive audio data from external sources
/// and forward it to internal graph components.
#[derive(Clone)]
pub struct SenderPort {
    format: AudioFormat,
    sender: Sender<Audio>
}

/// InputPort - это AudioOutput для аудио API
impl SenderPort {
    pub(crate) fn new(format: AudioFormat, output_tx: Sender<Audio>) -> Self {
        Self {
            format,
            sender: output_tx,
        }
    }

    pub(crate) fn sender(&self) -> Sender<Audio> {
        self.sender.clone()
    }
}

impl AudioOutput for SenderPort {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
        Ok(Box::pin(PollSender::new(self.sender.clone())
            .sink_map_err(|_| CoreError::Transport(TransportError::ConnectionClosed))))
    }

    fn start(&mut self) -> crate::core::error::Result<()> {
        Ok(())
    }

    fn stop(&mut self) -> crate::core::error::Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, AudioFormat, AudioInput};
    use crate::core::error::Result;
    use crate::runtime::ReceiverPort;
    use futures_util::StreamExt;

    fn test_format() -> AudioFormat {
        AudioFormat::from(crate::audio::EncodedAudioFormat::internal_format())
    }

    #[test]
    fn new_stores_format() {
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let port = SenderPort::new(test_format(), tx);
        assert_eq!(port.format(), test_format());
    }

    #[tokio::test]
    async fn sender_forwards_audio_to_receiver() -> Result<()> {
        let format = test_format();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let mut port = SenderPort::new(format.clone(), tx);
        let mut receiver = ReceiverPort::new(format.clone(), rx);

        let audio = Audio::from_pcm(
            &crate::audio::PcmAudio::new(
                symphonia::core::audio::AudioSpec::new(
                    format.sample_rate,
                    symphonia::core::audio::Channels::Discrete(format.channels),
                ),
                4,
            ),
        )
        .unwrap();

        port.sender().send(audio.clone()).await.unwrap();

        let mut stream = receiver.stream()?;
        let received = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await;
        assert!(received.is_ok());
        assert!(received.unwrap().is_some());
        Ok(())
    }
}
