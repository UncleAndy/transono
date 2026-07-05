use serde::{*};

use crate::providers::openai::realtime::protocol::{
    Audio,
    AudioFormat,
    AudioInput,
    AudioOutput,
    OutputModality,
    SessionConfig,
    TurnDetection};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ProtocolCommand {
    SessionUpdate(SessionUpdate),

    InputAudioBufferAppend(InputAudioBufferAppend),

    InputAudioBufferCommit(InputAudioBufferCommit),

    ResponseCreate(ResponseCreate),

    ResponseCancel(ResponseCancel),
}

#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: SessionConfig,
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

                        /*
                        turn_detection: Some(TurnDetection {
                            detection_type: "server_vad",
                            prefix_padding_ms: 1000,
                            silence_duration_ms: 100,
                        }),
                         */
                        turn_detection: None,
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

#[derive(Debug, Serialize)]
pub struct InputAudioBufferAppend {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub audio: String,
}

impl InputAudioBufferAppend {
    pub fn new(
        audio: impl Into<String>,
    ) -> ProtocolCommand {
        ProtocolCommand::InputAudioBufferAppend(
            Self {
                event_type: "input_audio_buffer.append",
                audio: audio.into(),
            },
        )
    }
}

#[derive(Debug, Serialize)]
pub struct InputAudioBufferCommit {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl InputAudioBufferCommit {
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::InputAudioBufferCommit(
            Self {
                event_type: "input_audio_buffer.commit",
            },
        )
    }
}
#[derive(Debug, Serialize)]
pub struct ResponseCreate {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCreate {
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::ResponseCreate(
            Self {
                event_type: "response.create",
            },
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ResponseCancel {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl ResponseCancel {
    pub fn new() -> ProtocolCommand {
        ProtocolCommand::ResponseCancel(
            Self {
                event_type: "response.cancel",
            },
        )
    }
}
