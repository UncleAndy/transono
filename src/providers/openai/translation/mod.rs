//! OpenAI Translation API provider (speech-to-speech translation sessions).
//!
//! Used by the `transono` binary for streaming capture → translate → playback.
//! Wire format details live in `commands` / `events` / `protocol`.
//! Session orchestration is in [`TranslationSession`].

/// Translation API commands.
pub mod commands;
/// Translation API events.
pub mod events;
/// Translation API protocol implementation.
pub mod protocol;
/// Translation API provider implementation.
pub mod provider;
/// Translation API session management.
pub mod session;
/// Translation API configuration.
pub mod config;

pub use commands::*;
pub use events::*;
pub use protocol::*;
pub use provider::*;
pub use session::*;
pub use config::*;
