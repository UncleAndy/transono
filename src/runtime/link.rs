use tokio::sync::mpsc;
use crate::audio::AudioFormat;
use crate::runtime::{InputPort, OutputPort};

pub struct AudioLink {
    input: InputPort,
    output: OutputPort,
}

impl AudioLink {
    pub fn new(format: AudioFormat, capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);

        Self {
            input: InputPort::new(format, tx),
            output: OutputPort::new(format, rx),
        }
    }

    pub fn input_port(&self) -> &InputPort {
        &self.input
    }

    pub fn output_port(&self) -> &OutputPort {
        &self.output
    }
}
