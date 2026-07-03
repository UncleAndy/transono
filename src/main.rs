mod audio;
mod openai;

use anyhow::Result;
use crate::openai::realtime::RealtimeClient;

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let api_key =
        std::env::var("OPENAI_API_KEY")?;

    let mut client =
        RealtimeClient::connect(
            &api_key,
            "You are a realtime translator.",
        )
            .await?;

    println!("Connected.");

    loop {
        let event = client.next_event().await?;

        println!("{event:#?}");
    }
}
