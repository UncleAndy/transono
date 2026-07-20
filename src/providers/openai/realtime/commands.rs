//! Client command types for the OpenAI Realtime WebSocket API.

use serde::{*};
use bytes::Bytes;
use crate::core::transport::serialize_bytes_as_str;

use crate::providers::openai::realtime::protocol::SessionConfig;

/// Client → server Realtime events encoded as JSON text frames.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProtocolCommand {
    /// `session.update` — configure session audio, model, and instructions.
    SessionUpdate(SessionUpdateEvent),

    /// `input_audio_buffer.append` — stream base64 audio into the input buffer.
    InputAudioBufferAppend(InputAudioBufferAppend),

    /// `input_audio_buffer.commit` — commit the buffered input audio.
    InputAudioBufferCommit(InputAudioBufferCommit),

    /// `response.create` — request a model response (manual turn mode).
    ResponseCreate(ResponseCreate),

    /// `response.cancel` — cancel an in-flight response.
    ResponseCancel(ResponseCancel),
}

/// Payload for a `session.update` client event.
#[derive(Debug, Serialize)]
pub struct SessionUpdateEvent {
    /// Event type discriminator (`session.update`).
    #[serde(rename = "type")]
    pub event_type: &'static str,

    /// Session parameters to apply.
    pub session: SessionConfig,
}

impl SessionUpdateEvent {
    /// Build a [`ProtocolCommand::SessionUpdate`] with the given session config.
    pub fn new(
        session: SessionConfig,
    ) -> ProtocolCommand {
        ProtocolCommand::SessionUpdate(SessionUpdateEvent {
            event_type: "session.update",
            session,
        })
    }
}

/// Payload for an `input_audio_buffer.append` client event.
#[derive(Debug, Serialize)]
pub struct InputAudioBufferAppend {
    /// Event type discriminator (`input_audio_buffer.append`).
    #[serde(rename = "type")]
    pub event_type: &'static str,

    /// Base64-encoded audio bytes (serialized as a UTF-8 string).
    #[serde(serialize_with = "serialize_bytes_as_str")]
    pub audio: Bytes,
}

impl InputAudioBufferAppend {
    /// Build a [`ProtocolCommand::InputAudioBufferAppend`] from raw audio bytes.
    pub fn new(
        audio: impl Into<Bytes>,
    ) -> ProtocolCommand {
        ProtocolCommand::InputAudioBufferAppend(
            Self {
                event_type: "input_audio_buffer.append",
                audio: audio.into(),
            },
        )
    }
}

/// Payload for an `input_audio_buffer.commit` client event.
#[derive(Debug, Serialize)]
pub struct InputAudioBufferCommit {
    /// Event type discriminator (`input_audio_buffer.commit`).
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl InputAudioBufferCommit {
    /// Build a [`ProtocolCommand::InputAudioBufferCommit`].
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::InputAudioBufferCommit(
            Self {
                event_type: "input_audio_buffer.commit",
            },
        )
    }
}

/// Payload for a `response.create` client event.
#[derive(Debug, Serialize)]
pub struct ResponseCreate {
    /// Event type discriminator (`response.create`).
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCreate {
    /// Build a [`ProtocolCommand::ResponseCreate`].
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::ResponseCreate(
            Self {
                event_type: "response.create",
            },
        )
    }
}

/// Payload for a `response.cancel` client event.
#[derive(Debug, Serialize)]
pub struct ResponseCancel {
    /// Event type discriminator (`response.cancel`).
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCancel {
    /// Build a [`ProtocolCommand::ResponseCancel`].
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::ResponseCancel(
            Self {
                event_type: "response.cancel",
            },
        )
    }
}
