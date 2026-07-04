use serde::Serialize;

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
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
        _model: impl Into<String>,
        _instructions: impl Into<String>,
        _voice: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "session.update",
            session: Session {
                // session_type: Some("realtime"),
                session_type: None,

                // model: Some(model.into()),
                model: None,

                // instructions: Some(instructions.into()),
                instructions: None,

                audio: Audio {
                    input: Some(AudioInput {
                        /*
                        format: Some(AudioFormat {
                            format_type: "audio/pcm",
                            rate: 24_000,
                        }),
                         */
                        format: None,

                        /*
                        turn_detection: TurnDetection {
                            detection_type: "server_vad",
                            prefix_padding_ms: 1000,
                            silence_duration_ms: 100,
                        },
                         */
                        turn_detection: None,
                    }),

                    output: AudioOutput {
                        /*
                        format: Some(AudioFormat {
                            format_type: "audio/pcm",
                            rate: 24_000,
                        }),
                         */
                        format: None,

                        // voice: Some(voice.into()),
                        voice: None,

                        language: Some("en".to_string()),
                    },
                },

                // output_modalities: Some(vec![OutputModality::Audio]),
                output_modalities: None,
                // modalities: Some(vec![OutputModality::Audio]),
                modalities: None,
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

#[derive(Debug, Serialize)]
pub struct SessionInputAudioAppend<'a> {
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

impl<'a> SessionInputAudioAppend<'a> {
    pub fn new(audio: &'a str) -> Self {
        Self {
            event_type: "session.input_audio_buffer.append",
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
