use anyhow::Result;
use async_trait::async_trait;
use crate::core::session_event::SessionEvent;

#[async_trait]
pub trait Session: Send {
    async fn send_audio(
        &mut self,
        audio: &[i16],
    ) -> Result<()>;

    async fn next_event(
        &mut self,
    ) -> Result<SessionEvent>;

    async fn close(
        &mut self,
    ) -> Result<()>;
}
