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

    /// `response.text.delta` — incremental text chunk from the model response.
    #[serde(rename = "response.text.delta")]
    ResponseTextDelta {
        /// Text delta from the model response.
        delta: String,
    },

    /// `response.text.done` — model response text completed.
    #[serde(rename = "response.text.done")]
    ResponseTextDone {
        /// Full response text (may be empty if accumulated from deltas).
        text: String,
    },

    /// `conversation.item/created` — a new conversation item (e.g. transcribed input).
    #[serde(rename = "conversation.item/created")]
    ConversationItemCreated {
        /// The conversation item.
        item: ConversationItem,
    },

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

/// A conversation item from the OpenAI Realtime API.
#[derive(Debug, Deserialize)]
pub struct ConversationItem {
    /// Item id.
    pub id: String,
    /// Item type (e.g. `"message"`).
    #[serde(rename = "type")]
    pub item_type: String,
    /// Role (e.g. `"user"`, `"assistant"`).
    #[serde(default)]
    pub role: Option<String>,
    /// Content parts of the item.
    #[serde(default)]
    pub content: Vec<ConversationItemContent>,
}

/// Content part of a conversation item.
#[derive(Debug, Deserialize)]
pub struct ConversationItemContent {
    /// Content type identifier (e.g. `"input_audio"`, `"text"`).
    #[serde(rename = "type")]
    pub content_type: String,
    /// Transcribed text (when content type is `"input_audio"`).
    #[serde(default)]
    pub transcript: Option<String>,
    /// Text content (when content type is `"text"`).
    #[serde(default)]
    pub text: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_response_text_delta() {
        let json = r#"{
            "type": "response.text.delta",
            "delta": "Hello, world!",
            "response_id": "resp_123",
            "item_id": "item_456",
            "output_index": 0,
            "content_index": 0
        }"#;
        let event: ProtocolEvent = serde_json::from_str(json).unwrap();
        match event {
            ProtocolEvent::ResponseTextDelta { delta } => {
                assert_eq!(delta, "Hello, world!");
            }
            other => panic!("expected ResponseTextDelta, got {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_response_text_done() {
        let json = r#"{
            "type": "response.text.done",
            "text": "Hello, world!",
            "response_id": "resp_123",
            "item_id": "item_456",
            "output_index": 0,
            "content_index": 0
        }"#;
        let event: ProtocolEvent = serde_json::from_str(json).unwrap();
        match event {
            ProtocolEvent::ResponseTextDone { text } => {
                assert_eq!(text, "Hello, world!");
            }
            other => panic!("expected ResponseTextDone, got {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_conversation_item_created_with_transcript() {
        let json = r#"{
            "type": "conversation.item/created",
            "item": {
                "id": "msg_abc123",
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_audio",
                        "audio": "base64data==",
                        "transcript": "What is your greatest strength?"
                    }
                ]
            }
        }"#;
        let event: ProtocolEvent = serde_json::from_str(json).unwrap();
        match event {
            ProtocolEvent::ConversationItemCreated { item } => {
                assert_eq!(item.id, "msg_abc123");
                assert_eq!(item.role.as_deref(), Some("user"));
                assert_eq!(item.content.len(), 1);
                assert_eq!(item.content[0].content_type, "input_audio");
                assert_eq!(
                    item.content[0].transcript.as_deref(),
                    Some("What is your greatest strength?")
                );
            }
            other => panic!("expected ConversationItemCreated, got {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_conversation_item_created_text_content() {
        let json = r#"{
            "type": "conversation.item/created",
            "item": {
                "id": "msg_xyz",
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Here is my answer."
                    }
                ]
            }
        }"#;
        let event: ProtocolEvent = serde_json::from_str(json).unwrap();
        match event {
            ProtocolEvent::ConversationItemCreated { item } => {
                assert_eq!(item.role.as_deref(), Some("assistant"));
                assert_eq!(item.content[0].content_type, "text");
                assert!(item.content[0].transcript.is_none());
                assert_eq!(item.content[0].text.as_deref(), Some("Here is my answer."));
            }
            other => panic!("expected ConversationItemCreated, got {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_conversation_item_created_empty_content() {
        let json = r#"{
            "type": "conversation.item/created",
            "item": {
                "id": "msg_empty",
                "type": "message",
                "role": "user",
                "content": []
            }
        }"#;
        let event: ProtocolEvent = serde_json::from_str(json).unwrap();
        match event {
            ProtocolEvent::ConversationItemCreated { item } => {
                assert_eq!(item.content.len(), 0);
            }
            other => panic!("expected ConversationItemCreated, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_event_falls_through() {
        let json = r#"{"type": "some.unknown.event", "data": 42}"#;
        let event: ProtocolEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, ProtocolEvent::Unknown));
    }
}
