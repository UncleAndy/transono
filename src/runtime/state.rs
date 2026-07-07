#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineState {
    Created,
    Running,
    Stopping,
    Stopped,
}
