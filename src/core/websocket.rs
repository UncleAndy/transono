use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;

use crate::core::error::Result;

pub struct WebSocketTransport {
    write: Option<
        SplitSink<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
            Message,
        >,
    >,

    read: Option<
        SplitStream<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
        >,
    >,
}

impl WebSocketTransport {
    pub async fn connect(
        request: http::Request<()>,
    ) -> Result<Self> {
        todo!()
    }
}