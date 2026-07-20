//! Capture-side device abstraction ([`AudioInput`]).

use std::sync::Arc;
use futures_util::stream::BoxStream;
use crate::audio::{Audio, AudioFormat, LatencyStats};
use crate::core::error::Result;

/// Audio capture endpoint that yields a stream of [`Audio`] chunks.
///
/// Implementations must be `Send` so they can move across tasks. Prefer
/// pre-allocated buffers inside the capture path; do not allocate in the
/// realtime callback when the backend supports it.
pub trait AudioInput: Send {
    /// Take ownership of the capture stream (typically once).
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the stream cannot be
    /// created (device not started, already taken, or backend failure).
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>>;

    /// Start capturing from the underlying device.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the device fails to start.
    fn start(&self) -> Result<()>;

    /// Stop capturing.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the device fails to stop.
    fn stop(&self) -> Result<()>;

    /// Negotiated capture format.
    fn format(&self) -> AudioFormat;

    /// Attach shared latency counters (optional; default no-op).
    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {}
}
