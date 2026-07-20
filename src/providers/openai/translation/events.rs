//! Server event types for the OpenAI Translation WebSocket API.

use serde::Deserialize;
use crate::providers::openai::error::OpenAiError;

/// Server → client Translation events decoded from JSON text frames.
///
/// Only the subset used by this crate is modeled; unrecognized `type` values
/// map to [`ProtocolEvent::Unknown`].
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ProtocolEvent {
    /// `session.created` — session resource after connect.
    #[serde(rename = "session.created")]
    SessionCreated {
        /// Session metadata from the server.
        session: SessionInfo,
    },

    /// `session.updated` — session after a successful `session.update`.
    #[serde(rename = "session.updated")]
    SessionUpdated {
        /// Updated session metadata.
        session: SessionInfo,
    },

    /// `session.output_audio.delta` — base64 PCM chunk of translated speech.
    #[serde(rename = "session.output_audio.delta")]
    SessionOutputAudioDelta {
        /// Base64-encoded audio delta.
        delta: String,
    },

    /// `session.output_transcript.delta` — incremental translated transcript text.
    #[serde(rename = "session.output_transcript.delta")]
    SessionOutputTranscriptDelta {
        /// Transcript text delta (target language).
        delta: String,
    },

    /// `session.input_transcript.delta` — incremental source-language ASR text.
    #[serde(rename = "session.input_transcript.delta")]
    SessionInputTranscriptDelta {
        /// Transcript text delta (source language).
        delta: String,
    },

    /// `error` — API error payload.
    #[serde(rename = "error")]
    Error(OpenAiError),

    /// Any other server event type not modeled here.
    #[serde(other)]
    Unknown,
}

/// Minimal session identity fields from Translation session events.
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    /// Server-assigned session id.
    pub id: String,

    /// Model associated with the session, when present.
    #[serde(default)]
    pub model: Option<String>,
}
