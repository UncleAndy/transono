use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::audio::{Audio, Pipelines};
use crate::core::error::Result;

pub trait ProviderSession {
    fn spawn(
        self,
        capture_rx: mpsc::Receiver<Audio>,
        playback_tx: mpsc::Sender<Audio>,
        pipelines: Pipelines,
        cancel: CancellationToken,
    ) -> JoinHandle<Result<Pipelines>>;
}

#[async_trait]
pub trait Provider {
    type Session: ProviderSession;

    async fn create_session(
        &self,
    ) -> Result<Self::Session>;
}
