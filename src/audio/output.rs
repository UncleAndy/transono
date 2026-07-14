use std::sync::Arc;
use std::pin::Pin;
use futures_util::Sink;
use crate::audio::{Audio, AudioFormat, LatencyStats};
use crate::core::error::{CoreError, Result};

pub type BoxSink<'a, T, E> = Pin<Box<dyn Sink<T, Error = E> + Send + 'a>>;

pub trait AudioOutput: Send {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn format(&self) -> AudioFormat;
    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {}
}
