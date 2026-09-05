use std::sync::Arc;
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::PollSender;
use futures_util::SinkExt;
use crate::audio::output::BoxSink;
use crate::audio::{sample_to_pcm_format, Audio, AudioFormat, AudioOutput, LatencyStats};
use crate::core::error::{CoreError, Result};

/// Audio output implementation using the CPAL library.
pub struct AudioOutputCpal {
    #[allow(unused)]
    name: String,
    stream: Stream,
    format: AudioFormat,
    sender: Option<Sender<Audio>>,
    stats: Arc<LatencyStats>,
}

struct OutputStateCpal<T> {
    current: Option<Audio>,
    current_samples: Vec<T>,
    offset: usize,
}

impl Drop for AudioOutputCpal {
    fn drop(&mut self) {
        let _ = self.stream.pause();
    }
}

impl AudioOutputCpal {
    /// Creates a new CPAL audio output for the specified device.
    pub fn new(
        device: Device,
        stats: Arc<LatencyStats>,
    ) -> Result<Self> {
        let (config, sample_format) = select_config_with_fallback(&device)?;

        let (tx, rx) = mpsc::channel(256);

        let stream = match sample_format {
            SampleFormat::F32 => {
                Self::build_stream::<f32>(
                    &device,
                    &config,
                    rx,
                    stats.clone(),
                )?
            }

            SampleFormat::I16 => {
                Self::build_stream::<i16>(
                    &device,
                    &config,
                    rx,
                    stats.clone(),
                )?
            }

            SampleFormat::U16 => {
                Self::build_stream::<u16>(
                    &device,
                    &config,
                    rx,
                    stats.clone(),
                )?
            }

            _ => {
                return Err(CoreError::Internal(
                "Unsupported sample format".to_string()
            ));
        }
        };

        Ok(Self {
            name: device.to_string(),
            stream,
            format: AudioFormat {
                sample_rate: config.sample_rate,
                channels: config.channels,
                sample_format: sample_to_pcm_format(sample_format),
            },
            sender: Some(tx),
            stats,
        })
    }

    fn build_stream<T>(
        device: &Device,
        config: &StreamConfig,
        mut receiver: mpsc::Receiver<Audio>,
        stats: Arc<LatencyStats>,
    ) -> Result<Stream>
    where
        T: cpal::SizedSample
        + symphonia::core::audio::conv::ConvertibleSample
        + Send
        + 'static,
    {
        let mut state = OutputStateCpal {
            current: None,
            current_samples: Vec::<T>::new(),
            offset: 0,
        };

        let stream = device.build_output_stream::<T, _, _>(
            *config,
            move |output: &mut [T], _| {
                output.fill(T::EQUILIBRIUM);

                let mut output_offset = 0;

                while output_offset < output.len() {
                    // Если текущий пакет закончился — взять следующий.
                    if state.current.is_none() {
                        state.current = receiver.try_recv().ok();

                        let Some(audio) = &state.current else {
                            stats.inc_dropped_output();
                            break;
                        };

                        state.current_samples.clear();

                        audio
                            .buffer()
                            .copy_to_vec_interleaved(
                                &mut state.current_samples,
                            );

                        state.offset = 0;
                    }

                    let remain =
                        &state.current_samples[state.offset..];

                    let count = remain
                        .len()
                        .min(output.len() - output_offset);

                    output[
                        output_offset..
                            output_offset + count
                        ]
                        .copy_from_slice(
                            &remain[..count],
                        );

                    output_offset += count;
                    state.offset += count;

                    if state.offset >= state.current_samples.len() {
                        state.current = None;
                        state.current_samples.clear();
                        state.offset = 0;
                    }
                }
            },
            move |err| {
                eprintln!("playback: {err}");
            },
            None,
        )
            .map_err(|e| CoreError::Cpal(e.to_string()))?;

        Ok(stream)
    }
}

impl AudioOutput for AudioOutputCpal {
    #[inline]
    fn sink(&mut self) -> Result<BoxSink<'static, Audio, CoreError>> {
        let Some(sender) = self.sender.clone() else {
            return Err(CoreError::Internal("sender absent".to_string()))
        };

        Ok(Box::pin(PollSender::new(sender)
            .sink_map_err(|_| CoreError::Transport(crate::core::error::TransportError::ConnectionClosed))))
    }

    #[inline]
    fn start(&mut self) -> Result<()> {
        self.stream.play().map_err(|e| CoreError::Cpal(e.to_string()))?;
        Ok(())
    }
 
    #[inline]
    fn stop(&mut self) -> Result<()> {
        self.stream.pause().map_err(|e| CoreError::Cpal(e.to_string()))?;
        Ok(())
    }

    #[inline]
    fn format(&self) -> AudioFormat {
        self.format.clone()
    }

    fn set_stats(&mut self, stats: Arc<LatencyStats>) {
        self.stats = stats;
    }
}

/// Чистая сборка конфига выхода с фиксированным (малым) буфером.
/// Не обращается к устройству — тестируется напрямую.
fn build_fixed_config(sample_rate: u32, channels: u16) -> StreamConfig {
    StreamConfig {
        channels,
        sample_rate,
        buffer_size: BufferSize::Fixed(
            crate::audio::latency_config::latency_frames(sample_rate),
        ),
    }
}

/// Решает итоговый BufferSize: фикс-буфер, если устройство его приняло,
/// иначе дефолт. Чистая функция — тестируется напрямую.
fn resolve_buffer_size(fixed_accepted: bool, sample_rate: u32) -> BufferSize {
    if fixed_accepted {
        BufferSize::Fixed(crate::audio::latency_config::latency_frames(sample_rate))
    } else {
        BufferSize::Default
    }
}

fn select_config(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
    let cfg = device.default_output_config().map_err(|e| CoreError::Cpal(e.to_string()))?;

    let config = build_fixed_config(cfg.sample_rate(), cfg.channels());

    Ok((config, cfg.sample_format()))
}

/// Выбирает конфиг устройства, пробуя фиксированный буфер для низкой задержки
/// и откатываясь на дефолт, если устройство/хост его не принимает.
fn select_config_with_fallback(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
    let cfg = device.default_output_config().map_err(|e| CoreError::Cpal(e.to_string()))?;
    let sample_rate = cfg.sample_rate();

    let (mut config, sample_format) = select_config(device)?;

    // Проверяем, что устройство действительно принимает запрошенный фикс-буфер.
    if let Err(e) = device.build_output_stream::<f32, _, _>(
        config.clone(),
        |_: &mut [f32], _: &cpal::OutputCallbackInfo| {},
        |_| {},
        None,
    ) {
        eprintln!(
            "output: BufferSize::Fixed rejected ({}), fallback to Default",
            e
        );
        config.buffer_size = resolve_buffer_size(false, sample_rate);
    }

    Ok((config, sample_format))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_config_uses_small_fixed_buffer() {
        let cfg = build_fixed_config(48000, 2);
        match cfg.buffer_size {
            BufferSize::Fixed(n) => assert_eq!(n, 480),
            other => panic!("expected Fixed(480), got {other:?}"),
        }
        assert_eq!(cfg.sample_rate, 48000);
        assert_eq!(cfg.channels, 2);
    }

    #[test]
    fn fixed_config_at_44100() {
        let cfg = build_fixed_config(44100, 1);
        match cfg.buffer_size {
            BufferSize::Fixed(n) => assert_eq!(n, 441),
            other => panic!("expected Fixed(441), got {other:?}"),
        }
    }

    #[test]
    fn resolve_keeps_fixed_when_accepted() {
        let bs = resolve_buffer_size(true, 48000);
        match bs {
            BufferSize::Fixed(n) => assert_eq!(n, 480),
            other => panic!("expected Fixed(480), got {other:?}"),
        }
    }

    #[test]
    fn resolve_falls_back_to_default_when_rejected() {
        let bs = resolve_buffer_size(false, 48000);
        assert!(matches!(bs, BufferSize::Default));

        // И в другой частоте — дефолт без привязки к latency_frames.
        assert!(matches!(resolve_buffer_size(false, 16000), BufferSize::Default));
    }
}
