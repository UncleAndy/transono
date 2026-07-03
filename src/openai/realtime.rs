use anyhow::Result;

use crate::openai::audio::{base64_to_pcm16, pcm16_to_base64};
use crate::openai::{
    client::WsClient,
    events::ServerEvent,
    protocol::{InputAudioAppend, InputAudioCommit, ResponseCreate, SessionUpdate},
};
use crate::openai::protocol::SessionInputAudioAppend;

const REALTIME_URL: &str = "wss://api.openai.com/v1/realtime/translations?model=gpt-realtime-translate";

pub struct RealtimeClient {
    ws: WsClient,
}

impl RealtimeClient {
    pub async fn connect(api_key: &str, instructions: impl Into<String>) -> Result<Self> {
        let mut ws = WsClient::connect(REALTIME_URL, api_key).await?;

        let session = SessionUpdate::new("gpt-realtime-translate", instructions, "cedar");

        ws.send(&session).await?;

        Ok(Self { ws })
    }

    #[inline]
    pub async fn append_audio(&mut self, pcm16: &[i16]) -> Result<()> {
        let audio = pcm16_to_base64(pcm16);

        let event = InputAudioAppend::new(&audio);

        self.ws.send(&event).await
    }

    #[inline]
    pub async fn session_append_audio(&mut self, pcm16: &[i16]) -> Result<()> {
        let audio = pcm16_to_base64(pcm16);

        let event = SessionInputAudioAppend::new(&audio);

        self.ws.send(&event).await
    }

    pub async fn next_audio(&mut self) -> anyhow::Result<Option<Vec<i16>>> {
        loop {
            match self.next_event().await? {
                ServerEvent::ResponseOutputAudioDelta { delta } => {
                    return Ok(Some(base64_to_pcm16(&delta)?));
                }

                ServerEvent::ResponseOutputAudioDone => {
                    return Ok(None);
                }

                _ => {}
            }
        }
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
    pub async fn next_event(&mut self) -> Result<ServerEvent> {
        self.ws.recv().await
    }

    #[inline]
    pub async fn close(self) -> Result<()> {
        self.ws.close().await
    }
}
