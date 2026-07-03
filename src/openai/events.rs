use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "session.created")]
    SessionCreated {
        session: Session,
    },

    #[serde(rename = "session.updated")]
    SessionUpdated {
        session: Session,
    },

    #[serde(rename = "response.audio.delta")]
    ResponseAudioDelta {
        delta: String,
    },

    #[serde(rename = "response.audio.done")]
    ResponseAudioDone,

    #[serde(rename = "response.done")]
    ResponseDone,

    #[serde(rename = "error")]
    Error {
        error: ErrorInfo,
    },
}

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,

    pub model: String,
}

#[derive(Debug, Deserialize)]
pub struct ErrorInfo {
    pub message: String,

    #[serde(default)]
    pub code: Option<String>,
}
