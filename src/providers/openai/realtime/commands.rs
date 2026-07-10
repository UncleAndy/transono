use serde::{*};
use bytes::Bytes;
use crate::core::transport::serialize_bytes_as_str;

use crate::providers::openai::realtime::protocol::SessionConfig;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProtocolCommand {
    SessionUpdate(SessionUpdateEvent),

    InputAudioBufferAppend(InputAudioBufferAppend),

    InputAudioBufferCommit(InputAudioBufferCommit),

    ResponseCreate(ResponseCreate),

    ResponseCancel(ResponseCancel),
}

#[derive(Debug, Serialize)]
pub struct SessionUpdateEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: SessionConfig,
}

impl SessionUpdateEvent {
    pub fn new(
        session: SessionConfig,
    ) -> ProtocolCommand {
        ProtocolCommand::SessionUpdate(SessionUpdateEvent {
            event_type: "session.update",
            session,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct InputAudioBufferAppend {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    #[serde(serialize_with = "serialize_bytes_as_str")]
    pub audio: Bytes,
}

impl InputAudioBufferAppend {
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

#[derive(Debug, Serialize)]
pub struct InputAudioBufferCommit {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl InputAudioBufferCommit {
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::InputAudioBufferCommit(
            Self {
                event_type: "input_audio_buffer.commit",
            },
        )
    }
}
#[derive(Debug, Serialize)]
pub struct ResponseCreate {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCreate {
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::ResponseCreate(
            Self {
                event_type: "response.create",
            },
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseCancel {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCancel {
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::ResponseCancel(
            Self {
                event_type: "response.cancel",
            },
        )
    }
}
