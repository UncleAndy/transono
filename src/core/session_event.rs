pub enum SessionEvent {
    Audio(Vec<i16>),

    SpeechStarted,

    SpeechStopped,

    ResponseStarted,

    ResponseFinished,

    Error(anyhow::Error),
}