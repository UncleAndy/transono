pub enum ProviderCommand<'a> {
    UpdateSession(SessionConfig),
    AppendAudio(&'a [i16]),
    Commit,
    Cancel,
}
