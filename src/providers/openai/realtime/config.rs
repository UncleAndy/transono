//! Connection config for the OpenAI Realtime WebSocket API.

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
use crate::providers::openai::realtime::protocol::TurnDetection;

/// Connection and session defaults for the OpenAI Realtime WebSocket API.
///
/// Prefer [`Self::from_env`] when credentials come from the process environment.
///
/// # Examples
///
/// ```no_run
/// use transono::providers::openai::realtime::OpenAIRealtimeConfig;
///
/// # fn demo() -> transono::core::error::Result<()> {
/// let mut cfg = OpenAIRealtimeConfig::from_env()?;
/// cfg.with_model("gpt-realtime").with_voice("alloy");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAIRealtimeConfig {
    /// Bearer API key (`OPENAI_API_KEY` when loaded via [`Self::from_env`]).
    pub api_key: String,
    /// Realtime model id (for example `gpt-realtime`).
    pub model: String,

    /// WebSocket base URL (query `model=` is appended when building the request).
    pub endpoint: String,

    /// Optional `OpenAI-Organization` header value.
    pub organization: Option<String>,
    /// Optional `OpenAI-Project` header value.
    pub project: Option<String>,

    /// Extra HTTP headers merged into the handshake request.
    pub headers: HashMap<String, String>,

    /// Server-side turn detection settings sent in `session.update`.
    pub turn_mode: TurnDetection,
    /// Optional system instructions for the session.
    pub instructions: Option<String>,
    /// Optional TTS voice name for audio output.
    pub voice: Option<String>,
    /// If set, enables server-side ASR transcription of input audio
    /// with the given model (e.g. `"whisper-1"`). This emits
    /// `SessionEvent::InputText` events containing the transcribed
    /// user speech.
    pub transcription_model: Option<String>,
}

impl OpenAIRealtimeConfig {
    /// Build a config from environment variables.
    ///
    /// Reads `OPENAI_API_KEY` and defaults the endpoint/model to the public
    /// Realtime API with server VAD turn detection.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if `OPENAI_API_KEY` is missing.
    pub fn from_env() -> Result<Self> {
        let mut cfg = Self {
            model: "gpt-realtime".to_string(),
            endpoint: "wss://api.openai.com/v1/realtime".to_string(),
            turn_mode: TurnDetection::server_vad(),
            ..Self::default()
        };

        cfg.api_key = match env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => return Err(CoreError::Internal("OPENAI_API_KEY environment variable required!".to_string())),
        };

        cfg.endpoint = "wss://api.openai.com/v1/realtime".to_string();

        Ok(cfg)
    }

    /// Override the Realtime model id.
    pub fn with_model(&mut self, model: &str) -> &mut Self {
        self.model = model.to_string();
        self
    }

    /// Set the output voice name.
    pub fn with_voice(&mut self, voice: &str) -> &mut Self {
        self.voice = Some(voice.to_string());
        self
    }

    /// Replace turn-detection settings used in `session.update`.
    pub fn with_turn_mode (&mut self, mode: TurnDetection) -> &mut Self {
        self.turn_mode = mode;
        self
    }

    /// Set optional session instructions.
    pub fn with_instructions(&mut self, instructions: &str) -> &mut Self {
        self.instructions = Some(instructions.to_string());
        self
    }

    /// Enable input-audio transcription (speech-to-text) with the given model.
    ///
    /// When enabled, the server emits `SessionEvent::InputText` events
    /// containing the transcribed user speech.
    pub fn with_transcription(&mut self, model: &str) -> &mut Self {
        self.transcription_model = Some(model.to_string());
        self
    }
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

    /// Encoded PCM format expected by this Realtime session (24 kHz mono, base64).
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

/// High-level turn-taking mode for Realtime sessions.
#[derive(Debug,Clone, Serialize, Deserialize, Default)]
pub enum TurnMode {
    /// Client commits turns manually (`input_audio_buffer.commit` / `response.create`).
    Manual,

    /// Server voice-activity detection ends turns automatically.
    #[default]
    ServerVad,
}

impl TurnMode {
    /// Map this mode to optional [`TurnDetection`] wire settings.
    ///
    /// Returns `None` for [`TurnMode::Manual`], or server VAD defaults for
    /// [`TurnMode::ServerVad`].
    pub fn turn_detection(&self) -> Option<TurnDetection> {
        match self {
            TurnMode::Manual => None,
            TurnMode::ServerVad =>
                Some(TurnDetection::server_vad()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_transcription_sets_model() {
        let mut cfg = OpenAIRealtimeConfig {
            api_key: "test-key".to_string(),
            model: "gpt-4o-realtime".to_string(),
            ..Default::default()
        };
        assert!(cfg.transcription_model.is_none());

        cfg.with_transcription("whisper-1");
        assert_eq!(cfg.transcription_model.as_deref(), Some("whisper-1"));
    }

    #[test]
    fn test_with_transcription_overrides_previous() {
        let mut cfg = OpenAIRealtimeConfig {
            api_key: "test-key".to_string(),
            model: "gpt-4o-realtime".to_string(),
            ..Default::default()
        };
        cfg.with_transcription("whisper-1");
        assert_eq!(cfg.transcription_model.as_deref(), Some("whisper-1"));

        cfg.with_transcription("whisper-1");
        // Same value — still set.
        assert!(cfg.transcription_model.is_some());
    }

    #[test]
    fn test_default_config_has_no_transcription() {
        let cfg = OpenAIRealtimeConfig::default();
        assert!(cfg.transcription_model.is_none());
    }

    #[test]
    fn test_default_config_has_empty_api_key() {
        let cfg = OpenAIRealtimeConfig::default();
        assert!(cfg.api_key.is_empty());
        assert!(cfg.model.is_empty());
    }

    #[test]
    fn test_audio_format_is_24khz_mono_i16() {
        let cfg = OpenAIRealtimeConfig::default();
        let fmt = cfg.audio_format();

        // The realtime API uses 24 kHz, mono, 16-bit little-endian PCM.
        let spec = fmt.spec();
        assert_eq!(spec.rate(), 24_000);
        assert_eq!(spec.channels().count(), 1);
    }
}
