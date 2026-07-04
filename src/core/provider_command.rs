pub enum ProviderCommand<'a> {
    UpdateSession(SessionConfig),
    AppendAudio(AudioFrame<'a>),
    Commit,
    Cancel,
}
