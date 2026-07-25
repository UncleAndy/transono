use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio_util::sync::CancellationToken;
use tokio::task::JoinHandle;
use futures_util::StreamExt;
use crate::audio::{Audio, AudioFormat, AudioInput};
use crate::runtime::receiver_port::ReceiverPort;
use crate::runtime::sender_port::SenderPort;

/// A utility for linking audio graph components.
///
/// Provides a factory for creating paired [`SenderPort`] and [`ReceiverPort`]
/// instances connected via an asynchronous channel, and managing background links.
pub struct AudioLink {
    cancel: Option<CancellationToken>,
    join_handle: Option<JoinHandle<()>>,
    sender: Option<Sender<Audio>>,
    receiver: Option<Receiver<Audio>>,
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
        mut input: ReceiverPort,
        output: SenderPort,
    ) -> Self {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let stream_result = input.stream();
        let tx = output.sender();

        let join_handle = tokio::spawn(async move {
            let Ok(mut stream) = stream_result else {
                return;
            };

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        break;
                    }
                    opt_audio = stream.next() => {
                        let Some(audio) = opt_audio else {
                            break;
                        };
                        if tx.send(audio).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self {
            cancel: Some(cancel),
            join_handle: Some(join_handle),
            sender: Some(output.sender()),
            receiver: None,
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
        // Проверка: есть ли такой цикл
        if let Some(cancel) = self.cancel.take() {
            cancel.cancel();
            if let Some(handle) = self.join_handle.take() {
                handle.abort();
            }
        } else {
            // Если цикла нет - закрывать каналы в правильном порядке.
            // Правильный порядок: сначала закрываем отправку (sender), затем прием (receiver).
            drop(self.sender.take());
            drop(self.receiver.take());
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
        let link = AudioLink::new_link(format, 10, rx_port, tx_port);
        link.stop();
    }

    #[test]
    fn test_audio_link_drop_no_loop() {
        let format = AudioFormat::from(EncodedAudioFormat::internal_format());
        let (_tx_port, _rx_port) = AudioLink::new_ports(format, 10);
        let (tx, rx) = mpsc::channel(10);
        let link = AudioLink {
            cancel: None,
            join_handle: None,
            sender: Some(tx),
            receiver: Some(rx),
        };
        drop(link);
    }
}
