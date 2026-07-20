//! Playback-side device abstraction ([`AudioOutput`]).

use std::sync::Arc;
use std::pin::Pin;
use futures_util::Sink;
use crate::audio::{Audio, AudioFormat, LatencyStats};
use crate::core::error::{CoreError, Result};

/// Owned, `Send` sink of audio chunks used by provider sessions and lines.
pub type BoxSink<'a, T, E> = Pin<Box<dyn Sink<T, Error = E> + Send + 'a>>;

/// Audio playback endpoint that consumes a sink of [`Audio`] chunks.
///
/// Implementations must be `Send` so they can move across tasks.
pub trait AudioOutput: Send {
    /// Take ownership of the playback sink (typically once).
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the sink cannot be
    /// created (device not started, already taken, or backend failure).
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>>;

    /// Start playback on the underlying device.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the device fails to start.
    fn start(&mut self) -> Result<()>;

    /// Stop playback.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the device fails to stop.
    fn stop(&mut self) -> Result<()>;

    /// Negotiated playback format.
    fn format(&self) -> AudioFormat;

    /// Attach shared latency counters (optional; default no-op).
    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {}
}
