use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;
use crate::audio::{Audio, AudioFormat};
use crate::runtime::{InputPort, OutputPort};

#[allow(unused)]
struct AudioSplitter {
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
    pub fn input_port(&self) -> &InputPort {
        &self.input
    }

    #[allow(unused)]
    pub fn new_output(&mut self) -> &OutputPort {
        let (tx, rx) = mpsc::channel(self.capacity);

        let port = OutputPort::new(self.format, rx);

        self.outputs.push(port);
        self.outputs_tx.push(tx);

        self.outputs
            .last()
            .expect("just pushed")
    }

    #[allow(unused)]
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
