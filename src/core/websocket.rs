use futures_util::stream::SplitSink;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

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
