use futures_util::{
    SinkExt,
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    MaybeTlsStream,
    WebSocketStream,
    tungstenite::{
        Message,
        client::IntoClientRequest,
    },
};
use crate::core::{
    error::{CoreError, Result},
    transport::{Transport, TransportData},
};
use crate::core::error::TransportError;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Writer = SplitSink<Socket, Message>;
type Reader = SplitStream<Socket>;

pub struct WebSocketTransport {
    writer: Writer,
    reader: Reader,
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

        let (writer, reader) = socket.split();

        Ok(Self {
            writer,
            reader,
        })
    }
}

#[async_trait::async_trait]
impl Transport for WebSocketTransport {
    async fn send(
        &mut self,
        data: TransportData,
    ) -> Result<()> {
        let message = match data {
            TransportData::Text(text) => {
                Message::Text(text.into())
            }

            TransportData::Binary(data) => {
                Message::Binary(data.into())
            }
        };

        self.writer
            .send(message)
            .await
            .map_err(TransportError::from)?;

        Ok(())
    }

    async fn recv(
        &mut self,
    ) -> Result<TransportData> {
        loop {
            let message = self
                .reader
                .next()
                .await
                .ok_or(CoreError::Transport(TransportError::ConnectionClosed))?
                .map_err(TransportError::from)?;

            match message {
                Message::Text(text) => {
                    return Ok(text.to_string().into());
                }

                Message::Binary(data) => {
                    return Ok(data.to_vec().into());
                }

                Message::Ping(data) => {
                    self.writer.send(
                            Message::Pong(data)
                        ).await
                        .map_err(TransportError::from)?;
                    continue;
                }

                Message::Pong(_) => {
                    continue;
                }

                Message::Frame(_) => {
                    // Internal tungstenite frame representation.
                    continue;
                }

                Message::Close(_) => {
                    return Err(CoreError::Transport(TransportError::ConnectionClosed));
                }
            }
        }
    }
}
