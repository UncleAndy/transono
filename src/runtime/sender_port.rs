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
