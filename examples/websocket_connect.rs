use http::Request;
use realtime_translator::core::websocket::WebSocketTransport;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let request = Request::builder()
        .uri("wss://echo.websocket.events")
        .body(())?;

    let _transport = WebSocketTransport::connect(request).await?;

    println!("Connected!");

    Ok(())
}
