use anyhow::Result;
use dotenvy::dotenv;
use std::env;

use realtime_translator::core::provider::Provider;
use realtime_translator::providers::openai::realtime::{
    config::OpenAIRealtimeConfig,
    provider::OpenAIRealtimeProvider,
    protocol::SessionUpdate,
};


#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key = env::var("OPENAI_API_KEY")?;

    let config = OpenAIRealtimeConfig {
        api_key,
        model: "gpt-realtime".into(),
        endpoint: "wss://api.openai.com/v1/realtime".into(),

        organization: None,
        project: None,

        headers: Default::default(),
    };

    println!("Creating provider...");

    let provider = OpenAIRealtimeProvider::new(config);

    println!("Opening session...");

    let mut session = provider.create_session().await?;

    println!("Sending SessionUpdate...");

    let update = SessionUpdate::new(
        "gpt-realtime-translate",
        "You are a realtime translator.",
        "cedar",
    );

    session.send(update).await?;

    println!("SessionUpdate sent.");

    Ok(())
}
