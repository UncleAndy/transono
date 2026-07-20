//! High-level lifecycle and content events for a realtime session.

use crate::audio::Audio;

/// Event observed on a running session (status, audio, or text).
#[derive(Debug)]
pub enum SessionEvent {
    /// Remote session became ready; payload is a session identifier.
    SessionStarted(String),

    /// Session configuration was acknowledged; payload is a status or id.
    SessionConfigured(String),

    /// Next chunk of output audio from the provider.
    Audio(Audio),

    /// Next chunk of output text from the provider.
    Text(String),

    /// Next chunk of transcribed input text (ASR on the capture path).
    InputText(String),

    /// Provider started accepting or processing a new user request.
    RequestStarted,

    /// User request input is complete.
    RequestFinished,

    /// Provider began generating a response.
    ResponseStarted,

    /// Response generation finished and was fully delivered.
    ResponseFinished,
}
