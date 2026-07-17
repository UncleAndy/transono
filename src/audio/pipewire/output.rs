use futures_util::Sink;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::audio::{
    Audio, AudioBuffer, AudioFormat, AudioOutput, BoxSink, FRAME_CAPACITY, FrameProducer,
    LatencyStats, PipeWireWorker,
};
use crate::core::error::{CoreError, Result};

struct PipeWireSink {
    producer: FrameProducer,
    scratch: Vec<f32>,
}

impl Sink<Audio> for PipeWireSink {
    type Error = CoreError;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Audio) -> std::result::Result<(), Self::Error> {
        let this = self.get_mut();

        this.scratch.clear();

        let ok = this.producer.send_audio(item, &mut this.scratch)?;

        if !ok {
            // Потеря кадра
        }

        Ok(())
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub struct PipeWireOutput {
    producer: Option<FrameProducer>,
    format: AudioFormat,

    _node_name: String,
    _node_id: Option<u32>,

    _worker: Option<PipeWireWorker>,
}

impl PipeWireOutput {
    pub fn new(format: AudioFormat, node_name: String, node_id: u32) -> Self {
        let (producer, consumer) = AudioBuffer::new(32).unwrap();

        Self {
            producer: Some(producer),
            format,
            _node_name: node_name.clone(),
            _node_id: Some(node_id),
            _worker: Some(
                PipeWireWorker::spawn_output(consumer, format, node_name.clone(), Some(node_id))
                    .ok()
                    .unwrap(),
            ),
        }
    }
}

impl AudioOutput for PipeWireOutput {
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
        Ok(Box::pin(PipeWireSink {
            producer: self.producer.take().unwrap(),
            scratch: Vec::with_capacity(FRAME_CAPACITY),
        }))
    }

    fn start(&self) -> Result<()> {
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {
        // Noop
    }
}
