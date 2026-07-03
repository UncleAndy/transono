use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: Session,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    Pcm16,
    G711Ulaw,
    G711Alaw,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Audio,
}

#[derive(Debug, Serialize)]
pub struct Session {
    pub model: String,

    pub instructions: String,

    pub input_audio_format: AudioFormat,

    pub output_audio_format: AudioFormat,

    pub voice: String,

    pub modalities: Vec<Modality>,
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
                model: model.into(),
                instructions: instructions.into(),
                input_audio_format: AudioFormat::Pcm16,
                output_audio_format: AudioFormat::Pcm16,
                voice: voice.into(),
                modalities: vec![Modality::Audio],
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
