use std::sync::Arc;
use futures_util::stream::BoxStream;
use crate::audio::{Audio, AudioFormat, AudioInput, FrameConsumer, LatencyStats};

pub struct PipeWireInput {
    consumer: FrameConsumer,
    format: AudioFormat,
}

impl PipeWireInput {
    pub fn new(
        consumer: FrameConsumer,
        format: AudioFormat,
    ) -> Self {
        Self {
            consumer,
            format,
        }
    }
}

impl AudioInput for PipeWireInput {
    fn stream(&mut self) -> crate::core::error::Result<BoxStream<'static, Audio>> {
        todo!()
    }

    fn start(&self) -> crate::core::error::Result<()> {
        todo!()
    }

    fn stop(&self) -> crate::core::error::Result<()> {
        todo!()
    }

    fn format(&self) -> AudioFormat {
        todo!()
    }

    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {
        todo!()
    }
}
