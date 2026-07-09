use std::collections::HashMap;
use std::env;
use anyhow::anyhow;
use http::{HeaderName, HeaderValue};
use http::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use crate::audio::{AudioCodec, AudioContainer, BinaryEncoding, EncodedAudioFormat, Endianness, PcmFormat};
use crate::core::error::{CoreError, ProtocolError, Result};
use crate::providers::openai::translation::{AudioConfig, AudioFormat, AudioInput, AudioOutput, SessionConfig};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAITranslationConfig {
    pub api_key: String,
    pub model: String,

    pub endpoint: String,

    pub organization: Option<String>,
    pub project: Option<String>,

    pub headers: HashMap<String, String>,

    pub lang: String,
}

impl OpenAITranslationConfig {
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self {
            model: "gpt-realtime-translate".to_string(),
            endpoint: "wss://api.openai.com/v1/realtime/translations".to_string(),
            ..Self::default()
        };

        cfg.api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => return Err(CoreError::Other(anyhow!("OPENAI_API_KEY environment variable required!"))),
        };

        Ok(cfg)
    }

    pub fn with_model(&mut self, model: &str) -> &mut Self {
        self.model = model.to_string();
        self
    }

    pub fn with_lang(&mut self, lang: &str) -> &mut Self {
        self.lang = lang.to_string();
        self
    }
}

impl OpenAITranslationConfig {
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
            session_type: "realtime",
            model: self.model.clone(),

            audio: AudioConfig {
                input: Some(AudioInput {
                    format: Some(AudioFormat::pcm_24khz()),
                }),

                output: AudioOutput {
                    format: Some(AudioFormat::pcm_24khz()),
                },
            },
        }
    }
    pub fn audio_format(&self) -> EncodedAudioFormat {
        EncodedAudioFormat::new(
            AudioContainer::Raw,
            AudioCodec::Pcm(
                PcmFormat::I16(Endianness::Little),
            ),
            BinaryEncoding::Base64,
            AudioSpec::new(
                24_000,
                Channels::Positioned(Position::FRONT_CENTER),
            ),
        )
    }
}
