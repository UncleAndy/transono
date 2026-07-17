use std::sync::Arc;
use futures_util::stream::BoxStream;

use crate::audio::{Audio, AudioFormat, AudioInput, FrameConsumer, LatencyStats, PipeWireWorker};
use crate::core::error::Result;

pub struct PipeWireInput {
    consumer: FrameConsumer,
    format: AudioFormat,

    worker: Option<PipeWireWorker>,
}

impl PipeWireInput {
    pub fn new(
        consumer: FrameConsumer,
        format: AudioFormat,
    ) -> Self {
        Self {
            consumer,
            format,
            worker: None,
        }
    }
}

impl AudioInput for PipeWireInput {
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        todo!()
    }

    fn start(&self) -> Result<()> {
        todo!()
    }

    fn stop(&self) -> Result<()> {
        todo!()
    }

    fn format(&self) -> AudioFormat {
        todo!()
    }

    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {
        todo!()
    }
}
