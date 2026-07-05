use async_trait::async_trait;

use crate::audio::Audio;
use crate::core::error::Result;
use crate::core::session_event::SessionEvent;

#[async_trait]
pub trait Session: Send + Sync {
    async fn send_audio(
        &mut self,
        audio: Audio,
    ) -> Result<()>;

    async fn next_event(
        &mut self,
    ) -> Result<SessionEvent>;

    async fn close(
        &mut self,
    ) -> Result<()>;
}
