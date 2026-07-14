use async_trait::async_trait;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio::sync::mpsc;
use futures_util::stream::BoxStream;
use crate::audio::output::BoxSink;
use crate::audio::{Audio, Pipelines, EncodedAudioFormat};
use crate::core::error::{Result, CoreError};
use crate::core::session_event::SessionEvent;
 
pub trait ProviderSession {
    fn spawn(
        self,
        capture_stream: BoxStream<'static, Audio>,
        playback_sink: BoxSink<'static, Audio, CoreError>,
        pipelines: Pipelines,
        cancel: CancellationToken,
        event_tx: Option<mpsc::UnboundedSender<SessionEvent>>,
    ) -> JoinHandle<Result<Pipelines>>;
}

#[async_trait]
pub trait Provider {
    type Session: ProviderSession;

    async fn create_session(
        &self,
    ) -> Result<Self::Session>;

    fn audio_format(&self) -> EncodedAudioFormat;
}
