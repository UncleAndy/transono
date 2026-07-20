//! Client command types for the OpenAI Translation WebSocket API.

use serde::Serialize;
use bytes::Bytes;
use crate::core::transport::serialize_bytes_as_str;

use crate::providers::openai::translation::{SessionConfig};

/// Client → server Translation events encoded as JSON text frames.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProtocolCommand {
    /// `session.update` — configure session audio and target language.
    SessionUpdate(TranslationSessionUpdateEvent),

    /// `session.input_audio_buffer.append` — stream base64 audio into the input buffer.
    SessionInputAudioBufferAppend(SessionAudioBufferAppend),
}

/// Payload for a `session.update` client event.
#[derive(Debug, Serialize)]
pub struct TranslationSessionUpdateEvent {
    /// Event type discriminator (`session.update`).
    #[serde(rename = "type")]
    pub event_type: &'static str,

    /// Session parameters to apply.
    pub session: SessionConfig,
}

/// ASR model settings for optional input-audio transcription.
#[derive(Debug, Serialize)]
pub struct InputAudioTranscription {
    /// Transcription model id (for example `whisper-1`).
    pub model: String
}

impl TranslationSessionUpdateEvent {
    /// Build a [`ProtocolCommand::SessionUpdate`] with the given session config.
    pub fn new(
        session: SessionConfig,
    ) -> ProtocolCommand {
        ProtocolCommand::SessionUpdate(TranslationSessionUpdateEvent {
            event_type: "session.update",
            session,
        })
    }
}

/// Payload for a `session.input_audio_buffer.append` client event.
#[derive(Debug, Serialize)]
pub struct SessionAudioBufferAppend {
    /// Event type discriminator (`session.input_audio_buffer.append`).
    #[serde(rename = "type")]
    pub event_type: &'static str,

    /// Base64-encoded audio bytes (serialized as a UTF-8 string).
    #[serde(serialize_with = "serialize_bytes_as_str")]
    pub audio: Bytes,
}
