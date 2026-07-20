//! OpenAI Translation API provider (speech-to-speech translation sessions).
//!
//! Used by the `transono` binary for streaming capture → translate → playback.
//! Wire format details live in `commands` / `events` / `protocol`.
//! Session orchestration is in [`TranslationSession`].

pub mod commands;
pub mod events;
pub mod protocol;
pub mod provider;
pub mod session;
pub mod config;

pub use commands::*;
pub use events::*;
pub use protocol::*;
pub use provider::*;
pub use session::*;
pub use config::*;
