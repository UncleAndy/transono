//! Provider-agnostic core: transport, protocol, session, and errors.
//!
//! High-level orchestration ([`crate::line::TranslationLine`]) depends on
//! these abstractions, not on a specific AI vendor or wire format.

pub mod error;
pub mod protocol;
pub mod provider;
pub mod provider_command;
pub mod provider_event;
pub mod transport;
pub mod websocket;
pub mod session;
pub mod session_event;
