use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{client::IntoClientRequest, protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WsClient {
    stream: WsStream,
}

impl WsClient {
    pub async fn connect(url: &str, api_key: &str) -> Result<Self> {
        let mut request = url.into_client_request()?;

        request
            .headers_mut()
            .insert("Authorization", format!("Bearer {api_key}").parse()?);

        let config = WebSocketConfig::default();

        let (stream, _) = connect_async_tls_with_config(request, Some(config), false, None)
            .await
            .context("WebSocket connect failed")?;

        Ok(Self { stream })
    }

    pub async fn send<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;

        self.stream.send(Message::Text(json.into())).await?;

        Ok(())
    }

    pub async fn recv<T: DeserializeOwned>(&mut self) -> Result<T> {
        loop {
            let message = match self.stream.next().await.unwrap() {
                Ok(msg) => {
                    msg
                }
                Err(_) => {
                    anyhow::bail!("websocket closed");
                }
            };

            match message {
                Message::Text(text) => {
                    return Ok(serde_json::from_str(&text)?);
                }

                Message::Binary(_) => {
                }

                Message::Ping(data) => {
                    self.stream.send(Message::Pong(data)).await?;
                }

                Message::Pong(_) => {
                }

                Message::Close(frame) => {
                    anyhow::bail!("connection closed: {frame:?}");
                }

                _ => {
                }
            }
        }
    }

    pub async fn close(mut self) -> Result<()> {
        self.stream.close(None).await?;
        Ok(())
    }
}
