use serde::{Serialize};
use crate::core::protocol::Protocol;
use crate::core::error::{ProtocolError, Result};
use crate::core::transport::TransportData;
use crate::providers::openai::realtime::commands::ProtocolCommand;
use crate::providers::openai::realtime::events::ProtocolEvent;

#[derive(Default)]
pub struct RealtimeProtocol;

impl RealtimeProtocol {
    pub(crate) fn new() -> RealtimeProtocol {
        Self
    }
}

impl Protocol for RealtimeProtocol {
    type Command = ProtocolCommand;
    type Event = ProtocolEvent;

    const ENDPOINT: &'static str = "/v1/realtime";

    fn encode(&self, command: &Self::Command) -> Result<TransportData> {
        let json = serde_json::to_string(command)
            .map_err(|e| ProtocolError::Json(e))?;
        Ok(TransportData::Text(json))
    }

    fn decode(&self, data: TransportData) -> Result<Self::Event> {
        match data {
            TransportData::Text(text) => {
                Ok(serde_json::from_str(&text)
                    .map_err(|e| ProtocolError::Json(e))?)
            }

            TransportData::Binary(_) => {
                Err(ProtocolError::UnexpectedBinaryData.into())
            }
        }
    }
}


#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: SessionConfig,
}

#[derive(Debug, Serialize)]
pub struct SessionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub session_type: Option<&'static str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    pub audio: Audio,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<OutputModality>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<OutputModality>>,
}

#[derive(Debug, Serialize)]
pub struct Audio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AudioInput>,

    pub output: AudioOutput,
}

#[derive(Debug, Serialize)]
pub struct AudioInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TurnDetection>,
}

#[derive(Debug, Serialize)]
pub struct AudioOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AudioFormat {
    #[serde(rename = "type")]
    pub format_type: &'static str,

    pub rate: u32,
}

#[derive(Debug, Serialize)]
pub struct TurnDetection {
    #[serde(rename = "type")]
    pub detection_type: &'static str,

    pub prefix_padding_ms: u32,

    pub silence_duration_ms: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputModality {
    Audio,
    Text,
}

