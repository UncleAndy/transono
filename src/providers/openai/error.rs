//! OpenAI API error types shared by Realtime and Translation backends.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// OpenAI API error payload returned on the wire or mapped locally.
#[derive(Debug, Serialize, Deserialize, Error)]
pub enum OpenAiError {
    /// Unclassified OpenAI error message.
    #[error("OpenAI error: {0}")]
    Other(String),
}
