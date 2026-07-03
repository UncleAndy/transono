use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: Session,
}

#[derive(Debug, Serialize)]
pub struct Session {
    #[serde(rename = "type")]
    pub session_type: &'static str,

    pub model: String,

    pub instructions: String,

    pub audio: Audio,

    pub output_modalities: Vec<OutputModality>,
}

#[derive(Debug, Serialize)]
pub struct Audio {
    pub input: AudioInput,

    pub output: AudioOutput,
}

#[derive(Debug, Serialize)]
pub struct AudioInput {
    pub format: AudioFormat,

    pub turn_detection: TurnDetection,
}

#[derive(Debug, Serialize)]
pub struct AudioOutput {
    pub format: AudioFormat,

    pub voice: String,
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
                session_type: "realtime",

                model: model.into(),

                instructions: instructions.into(),

                audio: Audio {
                    input: AudioInput {
                        format: AudioFormat {
                            format_type: "audio/pcm",
                            rate: 24_000,
                        },

                        turn_detection: TurnDetection {
                            detection_type: "server_vad",
                            prefix_padding_ms: 1000,
                            silence_duration_ms: 400,
                        },
                    },

                    output: AudioOutput {
                        format: AudioFormat {
                            format_type: "audio/pcm",
                            rate: 24_000,
                        },

                        voice: voice.into(),
                    },
                },

                output_modalities: vec![OutputModality::Audio],
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InputAudioAppend<'a> {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub audio: &'a str,
}

impl<'a> InputAudioAppend<'a> {
    pub fn new(audio: &'a str) -> Self {
        Self {
            event_type: "input_audio_buffer.append",
            audio,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InputAudioCommit {
    #[serde(rename = "type")]
    pub event_type: &'static str,
}

impl InputAudioCommit {
    pub fn new() -> Self {
        Self {
            event_type: "input_audio_buffer.commit",
        }
    }
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
