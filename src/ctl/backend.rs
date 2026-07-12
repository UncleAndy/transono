use std::fmt::{Display, Formatter};
use crate::core::error::Result;

pub trait Backend {
    fn init(&self, lang: &str) -> Result<()>;

    fn remove(&self, lang: &str) -> Result<()>;

    fn list(&self) -> Result<Vec<DeviceInfo>>;

    fn status(&self) -> Result<Vec<DeviceStatus>>;

    fn doctor(&self) -> Result<DoctorReport>;
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub language: String,

    pub to_meeting_microphone: String,
    pub from_meeting_speaker: String,

    pub internal_to_meeting_speaker: String,
    pub internal_from_meeting_microphone: String,
}

impl Display for DeviceInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Language: {}", self.language)?;
        writeln!(f, "  Public:")?;
        writeln!(f, "    ToMeeting.Microphone     : {}", self.to_meeting_microphone)?;
        writeln!(f, "    FromMeeting.Speaker      : {}", self.from_meeting_speaker)?;
        writeln!(f, "  Internal:")?;
        writeln!(f, "    ToMeeting.Speaker        : {}", self.internal_to_meeting_speaker)?;
        writeln!(f, "    FromMeeting.Microphone   : {}", self.internal_from_meeting_microphone)
    }
}

#[derive(Debug, Clone)]
pub enum DeviceState {
    Present,
    Missing,
}

#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub name: String,
    pub state: DeviceState,
}

impl Display for DeviceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mark = match self.state {
            DeviceState::Present => "✔",
            DeviceState::Missing => "✘",
        };

        write!(f, "{mark} {}", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct DoctorItem {
    pub name: String,
    pub ok: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub items: Vec<DoctorItem>,
}

impl Display for DoctorReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for item in &self.items {
            let mark = if item.ok { "✔" } else { "✘" };

            writeln!(f, "{mark} {}", item.name)?;

            if let Some(details) = &item.details {
                writeln!(f, "    {details}")?;
            }
        }

        Ok(())
    }
}
