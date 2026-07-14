use tokio::sync::mpsc::Sender;
use tokio_util::sync::PollSender;
use futures_util::SinkExt;
use crate::audio::output::BoxSink;
use crate::audio::{Audio, AudioFormat, AudioOutput};
use crate::core::error::{CoreError, Result, TransportError};

#[derive(Clone)]
pub struct InputPort {
    format: AudioFormat,
    sender: Sender<Audio>
}

/// InputPort - это AudioOutput для аудио API
impl InputPort {
    pub(crate) fn new(format: AudioFormat, output_tx: Sender<Audio>) -> Self {
        Self {
            format,
            sender: output_tx,
        }
    }
}

impl AudioOutput for InputPort {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
        Ok(Box::pin(PollSender::new(self.sender.clone())
            .sink_map_err(|_| CoreError::Transport(TransportError::ConnectionClosed))))
    }

    fn start(&self) -> crate::core::error::Result<()> {
        Ok(())
    }

    fn stop(&self) -> crate::core::error::Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}
