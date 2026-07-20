//! WebSocket implementation of [`crate::core::transport::Transport`].
//!
//! Session and provider layers should depend on the [`Transport`] trait, not
//! on this concrete type, so alternate carriers remain interchangeable.

use futures_util::{
    SinkExt,
    StreamExt,
    stream::SplitStream,
};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    MaybeTlsStream,
    WebSocketStream,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        Utf8Bytes,
    },
};
use crate::core::{
    error::{CoreError, Result},
    transport::{Transport, TransportData},
};
use crate::core::error::TransportError;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Reader = SplitStream<Socket>;

/// WebSocket duplex backed by a writer task and an optional reader half.
pub struct WebSocketTransport {
    writer_tx: mpsc::Sender<Message>,
    reader: Option<Reader>,
}

impl WebSocketTransport {
    /// Dial `request` and split the socket into a background writer and local reader.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] (wrapped in [`CoreError`]) if the handshake
    /// or TCP/TLS connection fails.
    pub async fn connect<R>(
        request: R,
    ) -> Result<Self>
    where
        R: IntoClientRequest + Unpin,
    {
        let (socket, _) =
            connect_async(request)
                .await
                .map_err(TransportError::from)?;

        let (mut writer, reader) = socket.split();

        let (tx, mut rx) = mpsc::channel::<Message>(256);

        // Background task draining outbound frames to the socket.
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(_) = writer.send(msg).await {
                    break;
                }
            }
            let _ = writer.close().await;
        });

        Ok(Self {
            writer_tx: tx,
            reader: Some(reader),
        })
    }

    /// Clone the channel sender used by the background writer task.
    pub fn clone_sender(&self) -> mpsc::Sender<Message> {
        self.writer_tx.clone()
    }

    /// Take ownership of the reader half, leaving this transport write-only.
    pub fn take_reader(&mut self) -> Option<Reader> {
        self.reader.take()
    }
}

#[async_trait::async_trait]
impl Transport for WebSocketTransport {
    /// Enqueue a text or binary frame for the background writer.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::ConnectionClosed`] if the writer channel is gone.
    async fn send(
        &mut self,
        data: TransportData,
    ) -> Result<()> {
        let message = match data {
            TransportData::Text(data) => {
                Message::Text(unsafe { Utf8Bytes::from_bytes_unchecked(data) })
            }

            TransportData::Binary(data) => {
                Message::Binary(data)
            }
        };

        self.writer_tx
            .send(message)
            .await
            .map_err(|_| CoreError::Transport(TransportError::ConnectionClosed))?;

        Ok(())
    }

    /// Read the next application data frame, answering ping frames automatically.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::ConnectionClosed`] if the reader was taken or
    /// the peer closed, or other transport errors on I/O failure.
    async fn recv(
        &mut self,
    ) -> Result<TransportData> {
        let reader = self.reader.as_mut()
            .ok_or(CoreError::Transport(TransportError::ConnectionClosed))?;
        loop {
            let message = reader
                .next()
                .await
                .ok_or(CoreError::Transport(TransportError::ConnectionClosed))?
                .map_err(TransportError::from)?;

            match message {
                Message::Text(text) => {
                    return Ok(TransportData::Text(Bytes::copy_from_slice(text.as_bytes())));
                }

                Message::Binary(data) => {
                    return Ok(TransportData::Binary(data));
                }

                Message::Ping(data) => {
                    let _ = self.writer_tx.send(Message::Pong(data)).await;
                    continue;
                }

                Message::Pong(_) => {
                    continue;
                }

                Message::Frame(_) => {
                    continue;
                }

                Message::Close(_) => {
                    return Err(CoreError::Transport(TransportError::ConnectionClosed));
                }
            }
        }
    }

    /// Send a close frame and drain the reader briefly for a peer close.
    ///
    /// # Errors
    ///
    /// This implementation currently always returns `Ok(())` after best-effort
    /// teardown (writer/reader failures are ignored).
    async fn close(&mut self) -> Result<()> {
        let _ = self.writer_tx.send(Message::Close(None)).await;

        if let Some(mut reader) = self.reader.take() {
            let timeout = std::time::Duration::from_secs(2);
            let writer_tx = self.writer_tx.clone();
            let _ = tokio::time::timeout(timeout, async move {
                while let Some(message) = reader.next().await {
                    match message.map_err(TransportError::from) {
                        Ok(Message::Close(_)) => {
                            break;
                        }

                        Ok(Message::Ping(data)) => {
                            let _ = writer_tx.send(Message::Pong(data)).await;
                        }

                        _ => {}
                    }
                }
            }).await;
        }

        Ok(())
    }
}
