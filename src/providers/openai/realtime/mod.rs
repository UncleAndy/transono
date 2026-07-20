//! OpenAI Realtime API provider (bidirectional speech / tools).
//!
//! Wire format details live in `commands` / `events` / `protocol`.
//! Session orchestration is in [`RealtimeSession`].

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
