use std::collections::HashMap;
use std::env;
use http::{HeaderName, HeaderValue};
use http::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use symphonia::core::audio::{AudioSpec, Channels, Position};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request;
use crate::audio::{AudioCodec, AudioContainer, BinaryEncoding, EncodedAudioFormat, Endianness, PcmFormat};
use crate::core::error::{CoreError, ProtocolError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAITranslationConfig {
    pub api_key: String,

    pub endpoint: String,

    pub organization: Option<String>,
    pub project: Option<String>,

    pub headers: HashMap<String, String>,

    pub lang: String,
}

impl OpenAITranslationConfig {
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self {
            endpoint: "wss://api.openai.com/v1/realtime/translations".to_string(),
            ..Self::default()
        };

        cfg.api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => return Err(CoreError::Internal("OPENAI_API_KEY environment variable required!".to_string())),
        };

        Ok(cfg)
    }

    pub fn with_lang(&mut self, lang: &str) -> &mut Self {
        self.lang = lang.to_string();
        self
    }
}

impl OpenAITranslationConfig {
    pub(crate) fn request(&self) -> Result<Request> {
        let mut request = format!(
            "{}?model=gpt-realtime-translate",
            self.endpoint,
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
