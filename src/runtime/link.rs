use tokio::sync::mpsc;
use crate::audio::AudioFormat;
use crate::runtime::{InputPort, OutputPort};

pub struct AudioLink {}

impl AudioLink {
    pub fn new_ports(format: AudioFormat, capacity: usize) -> (InputPort, OutputPort) {
        let (tx, rx) = mpsc::channel(capacity);

        (InputPort::new(format, tx), OutputPort::new(format, rx))
    }
}
