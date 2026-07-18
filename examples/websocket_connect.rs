use futures_util::{SinkExt, StreamExt};

use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use transono::{
    core::{
        transport::Transport,
        websocket::WebSocketTransport,
    },
    testing::websocket_server::WebSocketTestServer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let server = WebSocketTestServer::start(|ws| async move {

        let (mut tx, mut rx) = ws.split();

        while let Some(msg) = rx.next().await {
            tx.send(msg?).await?;
        }

        Ok(())
    })
        .await?;

    let request = server.uri().into_client_request()?;

    let mut transport =
        WebSocketTransport::connect(request).await?;

    transport
        .send("Hello!".into())
        .await?;

    let response = transport.recv().await?;

    println!("{response:?}");

    Ok(())
}
