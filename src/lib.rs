//! Real-time speech translation library (TRANSONO).
//!
//! Builds streaming speech pipelines on top of AI providers. The main
//! application unit is a [`line::TranslationLine`]: capture → process →
//! provider session → playback.
//!
//! # Layers
//!
//! - [`core`] — transport, protocol, and provider abstractions
//! - [`providers`] — concrete AI backends (OpenAI Realtime / Translation)
//! - [`mod@line`] — one independent translation stream
//! - [`audio`] — devices, buffers, DSP pipeline
//! - [`runtime`] — experimental audio graph helpers
//! - [`ctl`] — OS virtual audio device management
//!
//! See also: `docs/architecture/` in the repository.

#![warn(missing_docs)]

/// Audio processing, devices, and formats.
pub mod audio;
/// Core protocol and session abstractions.
pub mod core;
/// Concrete AI provider implementations.
pub mod providers;
/// Internal testing utilities.
pub mod testing;
/// Audio graph and routing runtime.
pub mod runtime;
/// Console interface and logging.
pub mod console;
/// OS virtual audio device management.
pub mod ctl;
/// Translation line management.
pub mod line;
