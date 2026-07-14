use tokio::sync::mpsc::Receiver;
use futures_util::stream::BoxStream;
use tokio_stream::wrappers::ReceiverStream;
use futures_util::StreamExt;
use crate::audio::{Audio, AudioFormat, AudioInput};
use crate::core::error::{CoreError, Result};

/// OutputPort - это AudioInput для внешнего API
pub struct OutputPort {
    format: AudioFormat,
    receiver: Option<Receiver<Audio>>
}

impl OutputPort {
    pub(crate) fn new(format: AudioFormat, input_rx: Receiver<Audio>) -> Self {
        Self {
            format,
            receiver: Some(input_rx)
        }
    }
}

impl AudioInput for OutputPort {
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
