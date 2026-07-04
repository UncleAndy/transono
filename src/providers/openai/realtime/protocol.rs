use serde::Serialize;
use crate::core::protocol::Protocol;
use crate::core::error::Result;
use crate::providers::openai::realtime::commands::ProtocolCommand;
use crate::providers::openai::realtime::events::ProtocolEvent;
use crate::providers::openai::realtime::protocol;

#[derive(Default)]
pub struct RealtimeProtocol;

impl RealtimeProtocol {
    pub(crate) fn new() -> RealtimeProtocol {
        Self
    }
}

impl Protocol for RealtimeProtocol {
    type Command = ProtocolCommand<'static>;
    type Event = ProtocolEvent;

    const ENDPOINT: &'static str = "/v1/realtime";

    fn encode(&self, command: &Self::Command) -> Result<Vec<u8>> {
        todo!()
    }

    fn decode(&self, data: &[u8]) -> Result<Self::Event> {
        todo!()
    }
}

#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: Session,
}

#[derive(Debug, Serialize)]
pub struct Session {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub session_type: Option<&'static str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    pub audio: Audio,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<OutputModality>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<OutputModality>>,
}

#[derive(Debug, Serialize)]
pub struct Audio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AudioInput>,

    pub output: AudioOutput,
}

#[derive(Debug, Serialize)]
pub struct AudioInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<AudioFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TurnDetection>,
}

#[derive(Debug, Serialize)]
pub struct AudioOutput {
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

#[derive(Debug, Serialize)]
pub struct TurnDetection {
    #[serde(rename = "type")]
    pub detection_type: &'static str,

    pub prefix_padding_ms: u32,

    pub silence_duration_ms: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputModality {
    Audio,
    Text,
}

impl SessionUpdate {
    pub fn new(
        model: impl Into<String>,
        instructions: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "session.update",
            session: Session {
                session_type: Some("realtime"),

                model: Some(model.into()),

                instructions: Some(instructions.into()),

                audio: Audio {
                    input: Some(AudioInput {
                        format: Some(AudioFormat {
                            format_type: "audio/pcm",
                            rate: 24_000,
                        }),

                        turn_detection: Some(TurnDetection {
                            detection_type: "server_vad",
                            prefix_padding_ms: 1000,
                            silence_duration_ms: 100,
                        }),
                    }),

                    output: AudioOutput {
                        format: Some(AudioFormat {
                            format_type: "audio/pcm",
                            rate: 24_000,
                        }),

                        voice: Some(voice.into()),
                    },
                },

                output_modalities: Some(vec![OutputModality::Audio]),
                modalities: None,
            },
        }
    }
}
