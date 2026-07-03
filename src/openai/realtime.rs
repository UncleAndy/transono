use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::openai::{
    client::WsClient,
    events::ServerEvent,
    protocol::{
        InputAudioAppend,
        InputAudioCommit,
        ResponseCreate,
        SessionUpdate,
    },
};

const REALTIME_URL: &str =
    "wss://api.openai.com/v1/realtime?model=gpt-realtime";

pub struct RealtimeClient {
    ws: WsClient,
}

impl RealtimeClient {
    pub async fn connect(
        api_key: &str,
        instructions: impl Into<String>,
    ) -> Result<Self> {
        let mut ws =
            WsClient::connect(REALTIME_URL, api_key).await?;

        let session = SessionUpdate::new(
            "gpt-realtime",
            instructions,
            "alloy",
        );

        ws.send(&session).await?;

        Ok(Self { ws })
    }

    #[inline]
    pub async fn append_audio(
        &mut self,
        pcm16: &[i16],
    ) -> Result<()> {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                pcm16.as_ptr() as *const u8,
                pcm16.len() * 2,
            )
        };

        let audio = STANDARD.encode(bytes);

        let event = InputAudioAppend::new(&audio);

        self.ws.send(&event).await
    }

    #[inline]
    pub async fn commit_audio(&mut self) -> Result<()> {
        self.ws.send(&InputAudioCommit::new()).await
    }

    #[inline]
    pub async fn create_response(&mut self) -> Result<()> {
        self.ws.send(&ResponseCreate::new()).await
    }

    #[inline]
    pub async fn next_event(
        &mut self,
    ) -> Result<ServerEvent> {
        self.ws.recv().await
    }

    #[inline]
    pub async fn close(self) -> Result<()> {
        self.ws.close().await
    }
}
