use tokio_util::sync::CancellationToken;
use crate::audio::{AudioFormat, AudioInput};
use crate::runtime::AudioLink;
use crate::runtime::receiver_port::ReceiverPort;
use crate::runtime::sender_port::SenderPort;

#[allow(unused)]
/// A component for splitting one audio stream into multiple outputs.
///
/// Clones incoming audio frames and broadcasts them to all registered outputs.
pub struct AudioSplitter {
    cancel: CancellationToken,

    input: Box<dyn AudioInput>,
    format: AudioFormat,
    capacity: usize,

    outputs: Vec<SenderPort>,
}

impl AudioSplitter {
    #[allow(unused)]
    /// Creates a new audio splitter with the specified format and capacity.
    ///
    /// # Arguments
    ///
    /// * `format` - The [`AudioFormat`] for all input and output streams.
    /// * `capacity` - The buffer capacity for each output channel.
    pub fn new(format: AudioFormat, capacity: usize, input: Box<dyn AudioInput>) -> Self {
        Self {
            input,
            cancel: CancellationToken::new(),
            outputs: Vec::new(),
            format,
            capacity,
        }
    }

    #[allow(unused)]
    /// Creates a new output port and returns a reference to it.
    ///
    /// # Returns
    ///
    /// Returns a reference to the newly created [`OutputPort`].
    pub fn create_output(&mut self) -> Box<ReceiverPort> {
        let (link_sender_port, link_receiver_port) =
            AudioLink::new_ports(self.format, self.capacity);

        self.outputs.push(link_sender_port);

        Box::new(link_receiver_port)
    }

    #[allow(unused)]
    /// Starts the splitter's processing loop in a background task.
    pub fn start(&mut self) {
        todo!()
    }
}
