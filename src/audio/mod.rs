//! Audio devices, buffers, encoding, and DSP pipelines.
//!
//! Hot paths should avoid allocation: prefer pooled frames/PCM and
//! slice-based processing. Device backends: [`cpal`], [`pipewire`].

/// High-level audio buffer management and lock-free queues.
pub mod audio_buffer;
/// Audio frame representation and constants.
pub mod frame;
/// Pre-allocated audio frame pool.
pub mod frame_pool;
/// Audio processing pipeline.
pub mod pipeline;
/// Generic audio processor traits.
pub mod processor;
/// Universal audio container and device layout.
pub mod audio;
/// Sample buffer abstractions.
pub mod sample_buffer;
// pub mod pcm_codec;
/// Collection of specific audio processors.
pub mod processors;
/// Planar sample buffer implementation.
pub mod planar_sample_buffer;
/// Encoded audio data and format definitions.
pub mod encoded_audio;
/// Internal PCM audio representation (DSP).
pub mod pcm_audio;
/// Pool for reusing PCM audio buffers.
pub mod pcm_pool;
/// Audio encoding and decoding implementations.
pub mod encoders;
/// Audio encoder and decoder traits.
pub mod audio_encoder;
/// Diagnostic tools for audio processing.
pub mod diagnost;
/// Audio device abstractions and discovery.
pub mod device;
/// Audio input device management.
pub mod input;
/// Audio output device management.
pub mod output;
/// CPAL audio backend.
pub mod cpal;
/// PipeWire audio backend.
pub mod pipewire;

pub use audio_buffer::*;
pub use frame::*;
pub use frame_pool::*;
pub use pipeline::*;
pub use cpal::output_cpal::*;
pub use processor::*;
pub use audio::*;
pub use sample_buffer::*;
pub use planar_sample_buffer::*;
// pub use pcm_codec::*;
pub use encoded_audio::*;
pub use pcm_audio::*;
pub use pcm_pool::*;
pub use audio_encoder::*;
pub use device::*;
pub use input::*;
pub use output::*;
pub use cpal::*;
pub use pipewire::*;
