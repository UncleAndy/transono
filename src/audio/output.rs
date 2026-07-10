use tokio::sync::mpsc;

use crate::audio::{Audio, AudioFormat};
use crate::core::error::Result;

pub trait AudioOutput {
    fn clone_sender(&mut self) -> Result<mpsc::Sender<Audio>>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn format(&self) -> &AudioFormat;
}
