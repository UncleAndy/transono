use futures_util::{
    SinkExt,
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use http::Request;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    MaybeTlsStream,
    WebSocketStream,
    tungstenite::Message,
};

use crate::core::{
    error::{CoreError, Result},
    transport::{Transport, TransportData},
};
use crate::core::error::TransportError;
use crate::core::error::TransportError::ConnectionClosed;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
type Writer = SplitSink<Socket, Message>;
type Reader = SplitStream<Socket>;

pub struct WebSocketTransport {
    writer: Writer,
    reader: Reader,
}

impl WebSocketTransport {
    pub async fn connect(
        request: Request<()>,
    ) -> Result<Self> {
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
                .ok_or(CoreError::Transport(ConnectionClosed))?
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
                    continue;
                }

                Message::Close(_) => {
                    return Err(CoreError::Transport(ConnectionClosed));
                }
            }
        }
    }
}
