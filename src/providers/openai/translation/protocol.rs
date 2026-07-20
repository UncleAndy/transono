//! Wire protocol and session-config DTOs for OpenAI speech translation.

use serde::{Serialize};
use crate::core::protocol::Protocol;
use crate::core::error::{ProtocolError, Result};
use crate::core::transport::TransportData;
use crate::providers::openai::translation::{InputAudioTranscription, ProtocolCommand, ProtocolEvent};

/// JSON codec for OpenAI Translation client commands and server events.
///
/// Implements [`Protocol`] over text frames at `/v1/realtime/translations`.
/// Binary frames are rejected as unexpected for this API.
#[derive(Default, Clone)]
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
    /// Input/output audio configuration for the translation session.
    #[serde(rename = "audio")]
    pub audio: AudioConfig,
}

/// Nested audio input and output settings for a Translation session.
#[derive(Debug, Serialize)]
pub struct AudioConfig {
    /// Optional microphone / input path settings (format, transcription).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AudioInputConfig>,

    /// Speaker / output path settings including target language.
    pub output: AudioOutputConfig,
}

/// Input audio format and optional ASR transcription for the session.
#[derive(Debug, Serialize)]
pub struct AudioInputConfig {
    /// Encoded input format (for example PCM 24 kHz).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    /// Optional input-audio transcription (source-language ASR).
    #[serde(rename="transcription", skip_serializing_if = "Option::is_none")]
    pub input_audio_transcription: Option<InputAudioTranscription>,
}

/// Output audio format and target language for translated speech.
#[derive(Debug, Serialize)]
pub struct AudioOutputConfig {
    /// Encoded output format (for example PCM 24 kHz).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    /// Target language code for translated audio (for example `en`, `ru`).
    pub language: String,
}

/// Wire audio format (`type` + sample rate) for Translation I/O.
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
