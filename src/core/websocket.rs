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

pub struct WebSocketTransport {
    writer_tx: mpsc::Sender<Message>,
    reader: Option<Reader>,
}

impl WebSocketTransport {
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

        // Фоновая задача для отправки
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

    pub fn clone_sender(&self) -> mpsc::Sender<Message> {
        self.writer_tx.clone()
    }

    pub fn take_reader(&mut self) -> Option<Reader> {
        self.reader.take()
    }
}

#[async_trait::async_trait]
impl Transport for WebSocketTransport {
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
