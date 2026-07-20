//! OpenAI Realtime API provider (bidirectional speech / tools).
//!
//! Wire format details live in `commands` / `events` / `protocol`.
//! Session orchestration is in [`RealtimeSession`].

/// Realtime API commands.
pub mod commands;
/// Realtime API events.
pub mod events;
/// Realtime API protocol implementation.
pub mod protocol;
/// Realtime API provider implementation.
pub mod provider;
/// Realtime API session management.
pub mod session;
/// Realtime API configuration.
pub mod config;

pub use commands::*;
pub use events::*;
pub use protocol::*;
pub use provider::*;
pub use session::*;
pub use config::*;
