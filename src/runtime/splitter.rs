use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;
use crate::audio::{Audio, AudioFormat};
use crate::runtime::{InputPort, OutputPort};

#[allow(unused)]
/// A component for splitting one audio stream into multiple outputs.
///
/// Clones incoming audio frames and broadcasts them to all registered outputs.
pub struct AudioSplitter {
    cancel: CancellationToken,

    input: InputPort,
    format: AudioFormat,
    capacity: usize,

    input_rx: Receiver<Audio>,

    outputs: Vec<OutputPort>,
    outputs_tx: Vec<Sender<Audio>>
}

impl AudioSplitter {
    #[allow(unused)]
    /// Creates a new audio splitter with the specified format and capacity.
    ///
    /// # Arguments
    ///
    /// * `format` - The [`AudioFormat`] for all input and output streams.
    /// * `capacity` - The buffer capacity for each output channel.
    pub fn new(format: AudioFormat, capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);

        Self {
            cancel: CancellationToken::new(),
            input: InputPort::new(format, tx),
            outputs: Vec::new(),
            format,
            capacity,
            input_rx: rx,
            outputs_tx: Vec::new(),
        }
    }

    #[allow(unused)]
    /// Returns a reference to the splitter's input port.
    pub fn input_port(&self) -> &InputPort {
        &self.input
    }

    #[allow(unused)]
    /// Creates a new output port and returns a reference to it.
    ///
    /// # Returns
    ///
    /// Returns a reference to the newly created [`OutputPort`].
    pub fn create_output(&mut self) -> &OutputPort {
        let (tx, rx) = mpsc::channel(self.capacity);

        let port = OutputPort::new(self.format, rx);

        self.outputs.push(port);
        self.outputs_tx.push(tx);

        self.outputs
            .last()
            .expect("just pushed")
    }

    #[allow(unused)]
    /// Starts the splitter's processing loop in a background task.
    pub fn start(&mut self) {
        Self::spawn_run(
            self.cancel.clone(),
            std::mem::replace(&mut self.input_rx, mpsc::channel(1).1),
            self.outputs_tx.clone(),
        );
    }

    fn spawn_run(
        cancel: CancellationToken,
        mut input_rx: Receiver<Audio>,
        outputs_tx: Vec<Sender<Audio>>,
    ) {
        let outputs_tx = outputs_tx.clone();

        tokio::spawn(async move {
            loop {
                select! {
                    _ = cancel.cancelled() => {
                        break;
                    }

                    Some(audio) = input_rx.recv() => {
                        for tx in outputs_tx.iter() {
                            if tx.send(audio.clone()).await.is_err() {
                                // приемник закрыт
                                break;
                            }
                        }
                    }
                }
            }
        });
    }
}
