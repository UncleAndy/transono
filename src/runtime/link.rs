use tokio::sync::mpsc;
use crate::audio::AudioFormat;
use crate::runtime::{InputPort, OutputPort};

/// A utility for linking audio graph components.
///
/// Provides a factory for creating paired [`InputPort`] and [`OutputPort`]
/// instances connected via an asynchronous channel.
pub struct AudioLink {}

impl AudioLink {
    /// Creates a pair of connected input and output ports.
    ///
    /// The ports share the same [`AudioFormat`] and are backed by a channel
    /// with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `format` - The [`AudioFormat`] to be used by both ports.
    /// * `capacity` - The buffer capacity of the asynchronous channel connecting the ports.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing the [`InputPort`] and the [`OutputPort`].
    pub fn new_ports(format: AudioFormat, capacity: usize) -> (InputPort, OutputPort) {
        let (tx, rx) = mpsc::channel(capacity);

        (InputPort::new(format, tx), OutputPort::new(format, rx))
    }
}
