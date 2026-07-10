use serde::{Deserialize, Serialize};
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
        Ok(TransportData::Text(json.into()))
    }

    fn decode(&self, data: TransportData) -> Result<Self::Event> {
        match data {
            TransportData::Text(text) => {
                let s = std::str::from_utf8(text.as_ref())
                    .map_err(|e| ProtocolError::Other(e.to_string()))?;
                Ok(serde_json::from_str(s)
                    .map_err(|e| ProtocolError::Json(e))?)
            }

            TransportData::Binary(_) => {
                Err(ProtocolError::UnexpectedBinaryData.into())
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SessionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub session_type: Option<&'static str>,

    pub model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    pub audio: AudioConfig,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<OutputModality>>,
}

#[derive(Debug, Serialize)]
pub struct AudioConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AudioInputConfig>,

    pub output: AudioOutputConfig,
}

#[derive(Debug, Serialize)]
pub struct AudioInputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    pub turn_detection: Option<TurnDetection>,
}

#[derive(Debug, Serialize)]
pub struct AudioOutputConfig {
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

impl AudioFormat {
    pub fn pcm_24khz() -> Self {
        Self {
            format_type: "audio/pcm",
            rate: 24_000,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TurnDetection {
    #[serde(rename = "type")]
    pub detection_type: String,

    pub prefix_padding_ms: u32,

    pub silence_duration_ms: u32,
}

impl TurnDetection {
    pub fn server_vad() -> Self {
        Self {
            detection_type: "server_vad".to_string(),
            prefix_padding_ms: 300,
            silence_duration_ms: 200,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputModality {
    Audio,
    Text,
}

