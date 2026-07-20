//! Connection config for the OpenAI Realtime Translations WebSocket API.

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

/// Connection and session defaults for the OpenAI Translations WebSocket API.
///
/// Prefer [`Self::from_env`] when credentials come from the process environment.
/// Set the target language with [`Self::with_lang`] before opening a session.
///
/// # Examples
///
/// ```no_run
/// use transono::providers::openai::translation::OpenAITranslationConfig;
///
/// # fn demo() -> transono::core::error::Result<()> {
/// let mut cfg = OpenAITranslationConfig::from_env()?;
/// cfg.with_lang("ru");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAITranslationConfig {
    /// Bearer API key (`OPENAI_API_KEY` when loaded via [`Self::from_env`]).
    pub api_key: String,

    /// WebSocket base URL (query `model=gpt-realtime-translate` is appended when building the request).
    pub endpoint: String,

    /// Optional `OpenAI-Organization` header value.
    pub organization: Option<String>,
    /// Optional `OpenAI-Project` header value.
    pub project: Option<String>,

    /// Extra HTTP headers merged into the handshake request.
    pub headers: HashMap<String, String>,

    /// Target language code sent in `session.update` output settings (for example `en`, `ru`).
    pub lang: String,
}

impl OpenAITranslationConfig {
    /// Build a config from environment variables.
    ///
    /// Reads `OPENAI_API_KEY` and defaults the endpoint to the public
    /// Translations WebSocket API.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if `OPENAI_API_KEY` is missing.
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

    /// Set the target translation language code.
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

    /// Encoded PCM format expected by this Translation session (24 kHz mono, base64).
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
