use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Error)]
pub enum OpenAiError {
    #[error("OpenAI error: {0}")]
    Other(String),
}
