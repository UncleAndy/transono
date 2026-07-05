use std::collections::HashMap;
use http::{HeaderName, HeaderValue};
use http::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;

use crate::core::error::{ProtocolError, Result};
use crate::providers::openai::realtime::protocol::{Audio, AudioFormat, AudioInput, AudioOutput, OutputModality, SessionConfig, TurnDetection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIRealtimeConfig {
    pub api_key: String,
    pub model: String,

    pub endpoint: String,

    pub organization: Option<String>,
    pub project: Option<String>,

    pub headers: HashMap<String, String>,

    pub turn_mode: TurnMode,
    pub instructions: Option<String>,
    pub voice: Option<String>,
}

impl OpenAIRealtimeConfig {
    pub(crate) fn request(&self) -> Result<Request> {
        let mut request = format!(
            "{}?model={}",
            self.endpoint,
            self.model,
        )
            .into_client_request()
            .map_err(|e| ProtocolError::Other(e.to_string()))?;

        {
            let headers = request
                .headers_mut();

            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(
                    &format!("Bearer {}", self.api_key)
                )
                    .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
            );

            if let Some(org) = &self.organization {
                headers.insert(
                    "OpenAI-Organization",
                    HeaderValue::from_str(org)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }

            if let Some(project) = &self.project {
                headers.insert(
                    "OpenAI-Project",
                    HeaderValue::from_str(project)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }

            for (name, value) in &self.headers {
                headers.insert(
                    HeaderName::from_bytes(name.as_bytes())
                        .map_err(|e| ProtocolError::InvalidHeaderName(e))?,
                    HeaderValue::from_str(value)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }
        }

        Ok(request)
    }

    pub fn session(&self) -> SessionConfig {
        SessionConfig {
            session_type: Some("realtime"),
            model: Some(self.model.clone()),
            instructions: self.instructions.clone(),

            audio: Audio {
                input: Some(AudioInput {
                    format: Some(AudioFormat::pcm_24khz()),
                    turn_detection: self.turn_mode.turn_detection(),
                }),

                output: AudioOutput {
                    format: Some(AudioFormat::pcm_24khz()),
                    voice: self.voice.clone(),
                },
            },

            output_modalities: Some(vec![
                OutputModality::Audio,
            ]),
        }
    }
}

#[derive(Debug,Clone, Serialize, Deserialize)]
pub enum TurnMode {
    Manual,
    ServerVad,
}

impl TurnMode {
    pub fn turn_detection(&self) -> Option<TurnDetection> {
        match self {
            TurnMode::Manual => None,
            TurnMode::ServerVad =>
                Some(TurnDetection::server_vad()),
        }
    }
}
