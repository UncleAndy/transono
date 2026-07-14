use std::sync::Arc;
use futures_util::stream::BoxStream;
use crate::audio::{Audio, AudioFormat, LatencyStats};
use crate::core::error::Result;
 
pub trait AudioInput: Send {
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn format(&self) -> AudioFormat;
    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {}
}
