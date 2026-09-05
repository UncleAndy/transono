use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio::task::JoinHandle;
use futures_util::{SinkExt, StreamExt};

use crate::audio::{AudioFormat, AudioInput, AudioOutput};
use crate::runtime::receiver_port::ReceiverPort;
use crate::runtime::sender_port::SenderPort;
use crate::runtime::rtrb_ports::{new_rtrb_ports, RtrbReceiverPort, RtrbSenderPort};

/// A utility for linking audio graph components.
///
/// Provides a factory for creating paired [`SenderPort`] and [`ReceiverPort`]
/// instances connected via an asynchronous channel, and managing background links.
pub struct AudioLink {
    cancel: Option<CancellationToken>,
    join_handle: Option<JoinHandle<()>>,
    receiver: Box<dyn AudioInput>,
    sender: Box<dyn AudioOutput>,
}

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
    /// Returns a tuple containing the [`SenderPort`] and the [`ReceiverPort`].
    pub fn new_ports(format: AudioFormat, capacity: usize) -> (SenderPort, ReceiverPort) {
        let (tx, rx) = mpsc::channel(capacity);

        (SenderPort::new(format, tx), ReceiverPort::new(format, rx))
    }

    /// Creates a background loop copying data from `input` to `output`.
    ///
    /// # Arguments
    ///
    /// * `format` - The [`AudioFormat`] of the streams.
    /// * `capacity` - The buffer capacity for the link channel.
    /// * `input` - The input [`ReceiverPort`] providing the audio stream.
    /// * `output` - The output [`SenderPort`] receiving the audio stream.
    pub fn new_link(
        _format: AudioFormat,
        _capacity: usize,
        mut input: Box<dyn AudioInput>,
        mut output: Box<dyn AudioOutput>,
    ) -> Self {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let Ok(mut input_stream) = input.stream() else {
            panic!("Failed to create input stream");
        };
        let Ok(mut output_sink) = output.sink() else {
            panic!("Failed to create output sink");
        };

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        break;
                    }
                    opt_audio = input_stream.next() => {
                        let Some(audio) = opt_audio else {
                            break;
                        };
                        if output_sink.send(audio).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            cancel: Some(cancel),
            join_handle: Some(join_handle),
            receiver: input,
            sender: output,
        }
    }

    /// rtrb-backed variant of [`AudioLink::new_ports`].
    ///
    /// Returns an `(RtrbSenderPort, RtrbReceiverPort)` pair built on a lock-free
    /// SPSC ring instead of `tokio::sync::mpsc`. Use with [`AudioLink::new_link_rtrb`].
    pub fn new_ports_rtrb(format: AudioFormat, capacity: usize) -> (RtrbSenderPort, RtrbReceiverPort) {
        new_rtrb_ports(format, capacity)
    }

    /// rtrb-backed variant of [`AudioLink::new_link`].
    ///
    /// Copies data between the input (`AudioInput`) and output (`AudioOutput`)
    /// ports in a background task. The ports themselves carry the rtrb engine,
    /// so no `tokio::sync::mpsc` is involved on the hot path.
    pub fn new_link_rtrb(
        _format: AudioFormat,
        _capacity: usize,
        mut input: Box<dyn AudioInput>,
        mut output: Box<dyn AudioOutput>,
    ) -> Self {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let Ok(mut input_stream) = input.stream() else {
            panic!("Failed to create input stream");
        };
        let Ok(mut output_sink) = output.sink() else {
            panic!("Failed to create output sink");
        };

        let join_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        break;
                    }
                    opt_audio = input_stream.next() => {
                        let Some(audio) = opt_audio else {
                            break;
                        };
                        if output_sink.send(audio).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            cancel: Some(cancel),
            join_handle: Some(join_handle),
            receiver: input,
            sender: output,
        }
    }

    /// Stops the background link processing loop if running.
    pub fn stop(&self) {
        if let Some(cancel) = &self.cancel {
            cancel.cancel();
        }
    }
}

impl Drop for AudioLink {
    fn drop(&mut self) {
        // Проверка: есть ли рабочий цикл
        if let Some(cancel) = self.cancel.take() {
            cancel.cancel();
            if let Some(handle) = self.join_handle.take() {
                handle.abort();
            }
            let _ = self.sender.stop();
            let _ = self.receiver.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::EncodedAudioFormat;

    #[tokio::test]
    async fn test_audio_link_ports_and_link() {
        let format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let (tx_port, rx_port) = AudioLink::new_ports(format.clone(), 10);
        let link = AudioLink::new_link(format, 10, Box::new(rx_port), Box::new(tx_port));
        link.stop();
    }

    #[test]
    fn test_audio_link_drop_no_loop() {
        let format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let (_tx_port, _rx_port) = AudioLink::new_ports(format, 10);
        let link = AudioLink {
            cancel: None,
            join_handle: None,
            sender: Box::new(_tx_port),
            receiver: Box::new(_rx_port),
        };
        drop(link);
    }

    #[tokio::test]
    async fn test_audio_link_rtrb_ports_and_link() {
        let format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let (tx_port, rx_port) = AudioLink::new_ports_rtrb(format.clone(), 10);

        let (feed_tx, feed_rx) = AudioLink::new_ports(format.clone(), 4);
        let _feed_link = AudioLink::new_link(
            format.clone(),
            4,
            Box::new(rx_port),
            Box::new(feed_tx),
        );

        let link = AudioLink::new_link_rtrb(
            format,
            10,
            Box::new(feed_rx),
            Box::new(tx_port),
        );
        link.stop();
    }
}
