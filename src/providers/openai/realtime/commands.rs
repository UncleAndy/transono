use serde::{*};

use crate::providers::openai::realtime::protocol::{Audio, AudioFormat, AudioInput, AudioOutput, OutputModality, SessionConfig, SessionUpdate, TurnDetection};

#[derive(Debug, Serialize)]
pub enum ProtocolCommand {
    SessionUpdate(SessionUpdate),

    InputAudioBufferAppend {
        audio: String,
    },

    InputAudioBufferCommit,

    ResponseCreate(ResponseCreate),

    ResponseCancel,
}

#[derive(Debug, Serialize)]
pub struct ResponseCreate {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCreate {
    pub fn new() -> Self {
        Self {
            event_type: "response.create",
        }
    }
}

impl SessionUpdate {
    pub fn new(
        model: impl Into<String>,
        instructions: impl Into<String>,
        voice: impl Into<String>,
    ) -> ProtocolCommand {
        ProtocolCommand::SessionUpdate(SessionUpdate{
            event_type: "session.update",
            session: SessionConfig {
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
        })
    }
}
