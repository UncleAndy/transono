use std::fs;

use anyhow::Result;

use realtime_translator::openai::realtime::RealtimeClient;

#[tokio::main]
async fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    let api_key = std::env::var("OPENAI_API_KEY")?;

    let mut client = RealtimeClient::connect(
        &api_key,
        "You are a realtime translator.",
    )
        .await?;

    println!("Connected.");

    // Файл должен содержать RAW PCM16LE 24 kHz mono.
    let bytes = fs::read("sample.pcm")?;

    let pcm: Vec<i16> = bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();

    println!("Sending {} samples...", pcm.len());

    client.append_audio(&pcm).await?;
    client.commit_audio().await?;
    client.create_response().await?;

    let mut out = Vec::<i16>::new();

    while let Some(chunk) = client.next_audio().await? {
        println!("received {} samples", chunk.len());
        out.extend(chunk);
    }

    let mut bytes = Vec::with_capacity(out.len() * 2);

    for s in out {
        bytes.extend_from_slice(&s.to_le_bytes());
    }

    fs::write("output.pcm", bytes)?;

    println!("Done.");

    Ok(())
}
