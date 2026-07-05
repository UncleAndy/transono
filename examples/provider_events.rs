use anyhow::Result;
use dotenvy::dotenv;
use std::env;

use realtime_translator::core::provider::Provider;
use realtime_translator::providers::openai::realtime::{
    config::OpenAIRealtimeConfig,
    provider::OpenAIRealtimeProvider,
    commands::SessionUpdate,
};
use realtime_translator::providers::openai::realtime::config::TurnMode;

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

        turn_mode: TurnMode::Manual,

        instructions: Some("Ты - собеседник для ведения интересных разговоров.".to_string()),

        voice: Some("cedar".to_string()),
    };

    println!("Creating provider...");

    let provider = OpenAIRealtimeProvider::new(config.clone());

    println!("Opening session...");

    let mut session = provider.create_session().await?;

    println!("Sending SessionUpdate...");

    let update = SessionUpdate::new(config.session());

    session.send(update).await?;

    println!("SessionUpdate sent.");

    loop {
        let event = session.next_event().await?;

        println!("{event:#?}");
    }
}
