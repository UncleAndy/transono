use tokio::sync::mpsc::Sender;
use crate::audio::{Audio, AudioFormat, AudioOutput};

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
    fn clone_sender(&mut self) -> crate::core::error::Result<Sender<Audio>> {
        Ok(self.sender.clone())
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
