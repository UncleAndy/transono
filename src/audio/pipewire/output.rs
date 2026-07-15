use std::sync::Arc;

use crate::audio::{Audio, AudioFormat, AudioOutput, BoxSink, FrameProducer, LatencyStats};
use crate::core::error::{CoreError, Result};

pub struct PipeWireOutput {
    producer: FrameProducer,
    format: AudioFormat,
}

impl PipeWireOutput {
    pub fn new(
        producer: FrameProducer,
        format: AudioFormat,
    ) -> Self {
        Self {
            producer,
            format,
        }
    }
}

impl AudioOutput for PipeWireOutput {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
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
