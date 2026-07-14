use anyhow::anyhow;
use tokio::sync::mpsc::Receiver;

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
    fn take_receiver(&mut self) -> Result<Receiver<Audio>> {
        let Some(receiver) = self.receiver.take() else {
            return Err(CoreError::Other(anyhow!("receiver already taken")))
        };

        Ok(receiver)
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
