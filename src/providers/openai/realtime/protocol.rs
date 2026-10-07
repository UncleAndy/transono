//! Wire protocol and session-config DTOs for OpenAI Realtime.

use serde::{Deserialize, Serialize};
use crate::core::protocol::Protocol;
use crate::core::error::{ProtocolError, Result};
use crate::core::transport::TransportData;
use crate::providers::openai::realtime::commands::ProtocolCommand;
use crate::providers::openai::realtime::events::ProtocolEvent;

/// JSON codec for OpenAI Realtime client commands and server events.
///
/// Implements [`Protocol`] over text frames at `/v1/realtime`. Binary frames
/// are rejected as unexpected for this API.
#[derive(Default, Clone)]
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

/// Session parameters embedded in a `session.update` client event.
#[derive(Debug, Serialize)]
pub struct SessionConfig {
    /// Session kind (`"realtime"` for GA Realtime sessions).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub session_type: Option<&'static str>,

    /// Model id for the session.
    pub model: String,

    /// Optional system instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Optional server-side ASR transcription of input audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_transcription: Option<InputAudioTranscription>,

    /// Server-side turn detection (VAD) settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TurnDetection>,

    /// Input/output audio configuration.
    pub audio: AudioConfig,

    /// Optional output modalities (`audio`, `text`).
    #[serde(rename = "modalities")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<OutputModality>>,
}

/// ASR model configuration for `input_audio_transcription`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct InputAudioTranscription {
    /// Transcription model id (for example `"whisper-1"`).
    pub model: String,
}

/// Nested audio input and output settings for a Realtime session.
#[derive(Debug, Serialize)]
pub struct AudioConfig {
    /// Optional microphone / input path settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AudioInputConfig>,

    /// Speaker / output path settings.
    pub output: AudioOutputConfig,
}

/// Input audio format for the session.
#[derive(Debug, Serialize)]
pub struct AudioInputConfig {
    /// Encoded input format (for example PCM 24 kHz).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,
}

/// Output audio format and voice for model responses.
#[derive(Debug, Serialize)]
pub struct AudioOutputConfig {
    /// Encoded output format (for example PCM 24 kHz).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    /// Optional TTS voice name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
}

/// Wire audio format (`type` + sample rate) for Realtime I/O.
#[derive(Debug, Serialize)]
pub struct AudioFormat {
    /// MIME-like format type (for example `audio/pcm`).
    #[serde(rename = "type")]
    pub format_type: &'static str,

    /// Sample rate in Hz.
    pub rate: u32,
}

impl AudioFormat {
    /// PCM little-endian mono at 24 kHz, as used by this provider.
    pub fn pcm_24khz() -> Self {
        Self {
            format_type: "audio/pcm",
            rate: 24_000,
        }
    }
}

/// Server-side turn detection (voice activity detection) settings.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TurnDetection {
    /// Detection mode (for example `server_vad`).
    #[serde(rename = "type")]
    pub detection_type: String,

    /// Audio included before speech start, in milliseconds.
    pub prefix_padding_ms: u32,

    /// Silence duration that ends a turn, in milliseconds.
    pub silence_duration_ms: u32,
}

impl TurnDetection {
    /// Default `server_vad` parameters used by this crate.
    pub fn server_vad() -> Self {
        Self {
            detection_type: "server_vad".to_string(),
            prefix_padding_ms: 300,
            silence_duration_ms: 200,
        }
    }
}

/// Output modality requested from the model.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputModality {
    /// Audio response chunks.
    Audio,
    /// Text response chunks.
    Text,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_config_serializes_transcription_and_modalities() {
        let cfg = SessionConfig {
            session_type: Some("realtime"),
            model: "gpt-4o-realtime".to_string(),
            instructions: Some("You are a helpful assistant.".to_string()),
            input_audio_transcription: Some(InputAudioTranscription {
                model: "whisper-1".to_string(),
            }),
            turn_detection: Some(TurnDetection::server_vad()),
            audio: AudioConfig {
                input: Some(AudioInputConfig {
                    format: Some(AudioFormat::pcm_24khz()),
                }),
                output: AudioOutputConfig {
                    format: Some(AudioFormat::pcm_24khz()),
                    voice: Some("alloy".to_string()),
                },
            },
            output_modalities: Some(vec![OutputModality::Text, OutputModality::Audio]),
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Top-level fields are correctly serialized.
        assert_eq!(v["model"], "gpt-4o-realtime");
        assert_eq!(v["instructions"], "You are a helpful assistant.");
        assert_eq!(v["input_audio_transcription"]["model"], "whisper-1");
        assert_eq!(v["turn_detection"]["type"], "server_vad");
        assert_eq!(v["turn_detection"]["prefix_padding_ms"], 300);
        assert_eq!(v["turn_detection"]["silence_duration_ms"], 200);

        // Modalities are serialized as a JSON array at the top level.
        let modalities = v["modalities"].as_array().unwrap();
        assert_eq!(modalities.len(), 2);
        assert_eq!(modalities[0], "text");
        assert_eq!(modalities[1], "audio");

        // turn_detection is NOT nested inside audio.input anymore.
        assert!(v["audio"]["input"]["turn_detection"].is_null());
    }

    #[test]
    fn test_session_config_serializes_without_optional_fields() {
        let cfg = SessionConfig {
            session_type: None,
            model: "gpt-4o-realtime".to_string(),
            instructions: None,
            input_audio_transcription: None,
            turn_detection: None,
            audio: AudioConfig {
                input: None,
                output: AudioOutputConfig {
                    format: None,
                    voice: None,
                },
            },
            output_modalities: None,
        };

        let json = serde_json::to_string(&cfg).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Optional fields are skipped.
        assert!(v.get("type").is_none() || v["type"].is_null());
        assert!(v["instructions"].is_null());
        assert!(v["input_audio_transcription"].is_null());
        assert!(v["turn_detection"].is_null());
        assert!(v["modalities"].is_null());
        assert!(v["audio"]["input"].is_null());
    }

    #[test]
    fn test_output_modality_serde() {
        assert_eq!(
            serde_json::to_string(&OutputModality::Audio).unwrap(),
            "\"audio\""
        );
        assert_eq!(
            serde_json::to_string(&OutputModality::Text).unwrap(),
            "\"text\""
        );
    }

    #[test]
    fn test_turn_detection_server_vad_defaults() {
        let td = TurnDetection::server_vad();
        assert_eq!(td.detection_type, "server_vad");
        assert_eq!(td.prefix_padding_ms, 300);
        assert_eq!(td.silence_duration_ms, 200);
    }

    #[test]
    fn test_input_audio_transcription_default() {
        let iat = InputAudioTranscription::default();
        assert_eq!(iat.model, "");
    }
}
