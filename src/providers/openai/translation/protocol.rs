use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SessionUpdate {
    #[serde(rename = "type")]
    pub event_type: &'static str,

    pub session: Session,
}

#[derive(Debug, Serialize)]
pub struct Session {
    pub audio: Audio,
}

#[derive(Debug, Serialize)]
pub struct Audio {
    pub output: AudioOutput,
}

#[derive(Debug, Serialize)]
pub struct AudioOutput {
    pub language: Option<String>,
}
