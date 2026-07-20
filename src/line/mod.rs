//! One independent speech-translation stream ([`TranslationLine`]).
//!
//! A line owns audio I/O, DSP pipelines, and a provider session. Lines do
//! not know about each other; multi-party coordination belongs to a higher
//! layer (TranslationBridge — see architecture docs).

pub mod line;
pub mod state;

pub use line::*;
pub use state::*;
