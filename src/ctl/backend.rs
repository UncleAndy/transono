use std::fmt::{Display, Formatter};

use crate::core::error::Result;

/// Trait for managing virtual audio device backends.
pub trait Backend {
    /// Initializes the backend for a specific language.
    fn init(&self, lang: &str) -> Result<()>;

    /// Removes the virtual devices for a specific language.
    fn remove(&self, lang: &str) -> Result<()>;

    /// Returns the set of virtual devices for a specific language.
    fn devices(&self, lang: &str) -> Result<DeviceSet>;

    /// Returns the status of devices for a specific language.
    fn status(&self, lang: &str) -> Result<Vec<DeviceStatus>>;

    /// Performs a health check on the backend and returns a report.
    fn doctor(&self) -> Result<DoctorReport>;
}

/// A set of virtual audio devices used for bridging.
#[derive(Debug, Clone)]
pub struct DeviceSet {
    /// Public name of the microphone input for the meeting.
    pub to_meeting_microphone_in: String,
    /// Public name of the speaker output from the meeting.
    pub from_meeting_speaker_out: String,

    /// Internal name of the speaker output to the meeting.
    pub internal_to_meeting_speaker_out: String,
    /// Internal name of the microphone input from the meeting.
    pub internal_from_meeting_microphone_in: String,
}

impl Display for DeviceSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "  Public:")?;
        writeln!(f, "    ToMeeting.Microphone     : {}", self.to_meeting_microphone_in)?;
        writeln!(f, "    FromMeeting.Speaker      : {}", self.from_meeting_speaker_out)?;
        writeln!(f, "  Internal:")?;
        writeln!(f, "    ToMeeting.Speaker        : {}", self.internal_to_meeting_speaker_out)?;
        writeln!(f, "    FromMeeting.Microphone   : {}", self.internal_from_meeting_microphone_in)
    }
}

/// State of a virtual audio device.
#[derive(Debug, Clone)]
pub enum DeviceState {
    /// The device is present in the system.
    Present,
    /// The device is missing from the system.
    Missing,
}

/// Current status of a single virtual device.
#[derive(Debug, Clone)]
pub struct DeviceStatus {
    /// Name of the device.
    pub name: String,
    /// Current state of the device.
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

/// A single item in a diagnostic health report.
#[derive(Debug, Clone)]
pub struct DoctorItem {
    /// Name of the check performed.
    pub name: String,
    /// Whether the check passed successfully.
    pub ok: bool,
    /// Additional details about the check result.
    pub details: Option<String>,
}

/// A collection of diagnostic health check results.
#[derive(Debug, Clone)]
pub struct DoctorReport {
    /// List of report items.
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
