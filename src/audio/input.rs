use tokio::sync::mpsc;

use crate::audio::{Audio, AudioFormat};
use crate::core::error::Result;

pub trait AudioInput {
    fn take_receiver(&mut self) -> Result<mpsc::Receiver<Audio>>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn format(&self) -> &AudioFormat;
}
