use std::{future::Future, net::SocketAddr};

use anyhow::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio_tungstenite::{accept_async, WebSocketStream};

pub type TestWebSocket = WebSocketStream<TcpStream>;

pub struct WebSocketTestServer {
    addr: SocketAddr,
}

impl WebSocketTestServer {
    pub async fn start<F, Fut>(handler: F) -> Result<Self>
    where
        F: Fn(TestWebSocket) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let (ready_tx, ready_rx) = oneshot::channel();

        tokio::spawn(async move {
            let _ = ready_tx.send(());

            loop {
                let (stream, _) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => {
                        break;
                    }
                };

                let handler = handler.clone();

                tokio::spawn(async move {
                    let _ = Self::handle_connection(stream, handler).await;
                });
            }
        });

        ready_rx.await?;

        Ok(Self { addr })
    }

    async fn handle_connection<F, Fut>(
        stream: TcpStream,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(TestWebSocket) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        let websocket = accept_async(stream).await?;

        handler(websocket).await
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn uri(&self) -> String {
        format!("ws://{}", self.addr)
    }
}
