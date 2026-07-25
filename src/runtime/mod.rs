//! Audio graph and routing runtime.
//!
//! Provides components for mixing, splitting, and linking audio streams
//! in a flexible processing graph.

/// Components for linking audio processing nodes.
pub mod link;
/// Input port for audio data in the graph.
pub mod receiver_port;
/// Output port for audio data in the graph.
pub mod sender_port;
/// Splitter for distributing audio to multiple outputs.
pub mod splitter;
/// Mixer for combining multiple audio inputs.
pub mod mixer;

pub use link::*;
pub use sender_port::*;
pub use receiver_port::*;
#[allow(unused_imports)]
pub use splitter::*;
pub use mixer::*;
