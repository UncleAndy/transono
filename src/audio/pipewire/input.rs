use std::sync::Arc;
use std::task::Poll;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use symphonia::core::audio::{AudioMut, AudioBuffer as SymphoniaBuffer};

use crate::audio::{
    Audio, AudioBuffer, AudioFormat, AudioInput, FrameConsumer, LatencyStats, PipeWireWorker,
    IntoGenericBuffer,
};
use crate::core::error::{CoreError, Result};

/// Audio input implementation using PipeWire.
pub struct PipeWireInput {
    consumer: Option<FrameConsumer>,
    format: AudioFormat,

    _node_name: String,
    _node_id: u32,

    _worker: PipeWireWorker,
}

impl PipeWireInput {
    /// Creates a new PipeWire audio input for the specified node.
    pub fn new(format: AudioFormat, node_name: String, node_id: u32) -> Result<Self> {
        let (producer, consumer) = AudioBuffer::new(32)?;

        Ok(Self {
            consumer: Some(consumer),
            format,
            _node_name: node_name.clone(),
            _node_id: node_id,
            _worker: PipeWireWorker::spawn_input(producer, format, node_name, Some(node_id))?,
        })
    }
}

impl AudioInput for PipeWireInput {
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        let mut consumer = self.consumer.take().ok_or_else(|| {
            CoreError::Internal("stream() already called".into())
        })?;

        let format = self.format;

        let s = futures_util::stream::poll_fn(move |cx| {
            if let Some(frame_id) = consumer.receive() {
                let audio = consumer.read(frame_id, |frame| {
                    let samples = frame.samples();
                    let spec = format.spec();
                    let frames = samples.len() / spec.channels().count();

                    let mut buffer = SymphoniaBuffer::<f32>::new(spec, frames);
                    buffer.render_uninit(Some(frames));
                    buffer.copy_from_slice_interleaved::<f32, &[f32]>(&samples);

                    Audio::new(buffer.into_generic_buffer())
                });

                let _ = consumer.release(frame_id);
                Poll::Ready(Some(audio))
            } else {
                // В данной реализации мы используем wake_by_ref для упрощения.
                // В будущем можно добавить уведомление от PipeWireWorker.
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        });

        Ok(s.boxed())
    }

    fn start(&self) -> Result<()> {
        // PipeWire worker начинает работу сразу после spawn.
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        // В рамках AudioInput stop(&self) не позволяет вызвать shutdown(&mut self).
        // Однако Drop для PipeWireWorker выполнит необходимую очистку.
        Ok(())
    }

    fn format(&self) -> AudioFormat {
        self.format
    }

    fn set_stats(&mut self, _stats: Arc<LatencyStats>) {
        // Статистика пока не реализована для PipeWire входа.
    }
}
