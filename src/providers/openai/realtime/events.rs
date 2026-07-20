//! Server event types for the OpenAI Realtime WebSocket API.

use serde::Deserialize;
use crate::providers::openai::error::OpenAiError;

/// Server → client Realtime events decoded from JSON text frames.
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

    /// `response.output_audio.delta` — base64 PCM audio chunk.
    #[serde(rename = "response.output_audio.delta")]
    ResponseOutputAudioDelta {
        /// Base64-encoded audio delta.
        delta: String,
    },

    /// `response.output_audio.done` — end of audio for the current output item.
    #[serde(rename = "response.output_audio.done")]
    ResponseOutputAudioDone,

    /// `response.done` — model response completed.
    #[serde(rename = "response.done")]
    ResponseDone,

    /// `input_audio_buffer.speech_started` — server VAD detected speech start.
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted,

    /// `input_audio_buffer.speech_stopped` — server VAD detected speech end.
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped,

    /// `input_audio_buffer.committed` — input buffer was committed.
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted,

    /// `response.created` — a new model response started.
    #[serde(rename = "response.created")]
    ResponseCreated,

    /// `error` — API error payload.
    #[serde(rename = "error")]
    Error {
        /// OpenAI error body.
        error: OpenAiError,
    },

    /// Any other server event type not modeled here.
    #[serde(other)]
    Unknown,
}

/// Minimal session identity fields from Realtime session events.
#[derive(Debug, Deserialize)]
pub struct SessionInfo {
    /// Server-assigned session id.
    pub id: String,

    /// Model associated with the session, when present.
    #[serde(default)]
    pub model: Option<String>,
}
