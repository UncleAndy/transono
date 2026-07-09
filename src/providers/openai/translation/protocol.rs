use serde::{Serialize};
use crate::core::protocol::Protocol;
use crate::core::error::{ProtocolError, Result};
use crate::core::transport::TransportData;
use crate::providers::openai::translation::{ProtocolCommand, ProtocolEvent};

#[derive(Default)]
pub struct TranslationProtocol;

impl TranslationProtocol {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Protocol for TranslationProtocol {
    type Command = ProtocolCommand;
    type Event = ProtocolEvent;

    const ENDPOINT: &'static str = "/v1/realtime/translations";

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
pub struct SessionConfig {
    #[serde(rename = "audio")]
    pub audio: AudioConfig,
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
}

#[derive(Debug, Serialize)]
pub struct AudioOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    pub language: String,
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
