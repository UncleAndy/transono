use std::collections::HashMap;
use std::env;
use anyhow::anyhow;
use http::{HeaderName, HeaderValue};
use http::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use crate::audio::{AudioCodec, AudioContainer, BinaryEncoding, EncodedAudioFormat, Endianness};
use crate::core::error::{CoreError, ProtocolError, Result};
use crate::providers::openai::realtime::protocol::{Audio, AudioFormat, AudioInput, AudioOutput, OutputModality, SessionConfig, TurnDetection};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self {
            model: "gpt-realtime".to_string(),
            endpoint: "wss://api.openai.com/v1/realtime".to_string(),
            turn_mode: TurnMode::ServerVad,
            ..Self::default()
        };

        cfg.api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => return Err(CoreError::Other(anyhow!("OPENAI_API_KEY environment variable required!"))),
        };

        cfg.endpoint = "wss://api.openai.com/v1/realtime".to_string();

        Ok(cfg)
    }

    pub fn with_model(&mut self, model: &str) -> &mut Self {
        self.model = model.to_string();
        self
    }

    pub fn with_voice(&mut self, voice: &str) -> &mut Self {
        self.voice = Some(voice.to_string());
        self
    }

    pub fn with_turn_mode (&mut self, mode: TurnMode) -> &mut Self {
        self.turn_mode = mode;
        self
    }

    pub fn with_instructions(&mut self, instructions: &str) -> &mut Self {
        self.instructions = Some(instructions.to_string());
        self
    }
}

impl OpenAIRealtimeConfig {
    pub(crate) fn request(&self) -> Result<Request> {
        println!("DBG0: {:?}", &self);

        let mut request = format!(
            "{}?model={}",
            self.endpoint,
            self.model,
        )
            .into_client_request()
            .map_err(|e| ProtocolError::Other(e.to_string()))?;

        println!("DBG0.1");

        {
            println!("DBG1");

            let headers = request
                .headers_mut();

            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(
                    &format!("Bearer {}", self.api_key)
                )
                    .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
            );

            println!("DBG2");

            if let Some(org) = &self.organization {
                headers.insert(
                    "OpenAI-Organization",
                    HeaderValue::from_str(org)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }

            println!("DBG3");

            if let Some(project) = &self.project {
                headers.insert(
                    "OpenAI-Project",
                    HeaderValue::from_str(project)
                        .map_err(|e| ProtocolError::InvalidHeaderValue(e))?,
                );
            }

            println!("DBG4");

            for (name, value) in &self.headers {
                println!("DBG5: {} : {}", name, value);
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
    pub fn audio_format(&self) -> EncodedAudioFormat {
        EncodedAudioFormat::new(
            AudioContainer::Raw,
            AudioCodec::Pcm(Endianness::Little),
            BinaryEncoding::Base64,
            AudioSpec::new(
                24_000,
                Channels::Positioned(Position::FRONT_CENTER),
            ),
        )
    }
}

#[derive(Debug,Clone, Serialize, Deserialize, Default)]
pub enum TurnMode {
    Manual,

    #[default]
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
