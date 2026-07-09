use anyhow::Result;
use dotenvy::dotenv;
use std::env;

use realtime_translator::core::provider::Provider;
use realtime_translator::providers::openai::translation::{OpenAITranslationConfig, OpenAITranslationProvider};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let api_key = env::var("OPENAI_API_KEY")?;

    let config = OpenAITranslationConfig {
        api_key,
        model: "gpt-4o-realtime-preview".to_string(),

        endpoint: "wss://api.openai.com/v1/realtime".to_string(),

        organization: None,
        project: None,

        headers: Default::default(),

        lang: "en".to_string(),
    };

    println!("{:#?}", config);

    let provider = OpenAITranslationProvider::new(config);

    println!("Connecting...");

    let _session = provider.create_session().await?;

    println!("Connected successfully.");

    Ok(())
}
