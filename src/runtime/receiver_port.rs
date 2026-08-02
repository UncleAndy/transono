use tokio::sync::mpsc::Receiver;
use futures_util::stream::BoxStream;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use crate::audio::{Audio, AudioFormat, AudioInput};
use crate::core::error::{CoreError, Result};

/// An output exit point for an audio graph.
///
/// Implements [`AudioInput`] to provide processed audio data to
/// external consumers.
pub struct ReceiverPort {
    format: AudioFormat,
    receiver: Option<Receiver<Audio>>
}

impl ReceiverPort {
    pub(crate) fn new(format: AudioFormat, input_rx: Receiver<Audio>) -> Self {
        Self {
            format,
            receiver: Some(input_rx)
        }
    }
}

impl AudioInput for ReceiverPort {
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        let Some(receiver) = self.receiver.take() else {
            return Err(CoreError::Internal("receiver already taken".to_string()))
        };

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
    use crate::audio::{Audio, AudioFormat};
    use crate::core::error::Result;

    fn test_format() -> AudioFormat {
        AudioFormat::from(crate::audio::EncodedAudioFormat::internal_format())
    }

    #[test]
    fn new_stores_format() {
        let (_tx, rx) = tokio::sync::mpsc::channel(4);
        let port = ReceiverPort::new(test_format(), rx);
        assert_eq!(port.format(), test_format());
    }

    #[test]
    fn stream_take_twice_errors() {
        let (_tx, rx) = tokio::sync::mpsc::channel(4);
        let mut port = ReceiverPort::new(test_format(), rx);
        let _first = port.stream();
        // Second take must fail: receiver already consumed.
        assert!(port.stream().is_err());
    }

    #[tokio::test]
    async fn stream_yields_sent_audio() -> Result<()> {
        let format = test_format();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let mut port = ReceiverPort::new(format.clone(), rx);

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

        tx.send(audio.clone()).await.unwrap();

        let mut stream = port.stream()?;
        let received = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next()).await;
        assert!(received.is_ok());
        assert!(received.unwrap().is_some());
        Ok(())
    }
}
