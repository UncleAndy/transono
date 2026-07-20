//! Audio device discovery and open/create contracts.
//!
//! High-level orchestration depends on [`AudioDeviceFactory`] rather than a
//! concrete backend ([`crate::audio::cpal`], [`crate::audio::pipewire`]).

use crate::audio::{AudioFormat, AudioInput, AudioOutput};
use crate::core::error::Result;

/// Marker for a concrete audio endpoint implementation.
pub trait AudioDevice {}

/// Collection of opened input and output devices from a factory.
#[allow(unused)]
pub struct AudioDevices {
    input: Vec<Box<dyn AudioInput>>,
    output: Vec<Box<dyn AudioOutput>>,
}

/// Default object-safe device factory type alias.
pub type DefaultAudioDeviceFactory = dyn AudioDeviceFactory;

/// Factory that enumerates, opens hardware, and creates virtual devices.
#[allow(unused)]
pub trait AudioDeviceFactory {
    /// List available devices and their reported formats.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the backend cannot
    /// query the host device list.
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>>;

    /// Open hardware input/output devices described by `config`.
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if a requested device is
    /// missing or cannot be opened.
    fn open_hardware(
        &self,
        config: &HardwareDeviceConfig,
    ) -> Result<AudioDevices>;

    /// Create a virtual device pair (e.g. for meeting-app bridging).
    ///
    /// # Errors
    ///
    /// Returns a [`crate::core::error::CoreError`] if the virtual device
    /// cannot be created on the host.
    fn create_virtual(
        &self,
        config: &VirtualDeviceConfig,
    ) -> Result<AudioDevices>;
}

/// Parameters for creating a virtual audio device.
pub struct VirtualDeviceConfig {
    /// Stable identifier for the virtual device.
    pub id: AudioDeviceId,
    /// Human-readable device name.
    pub name: String,
    /// Sample rate in Hz.
    pub sample_rate: SampleRate,
    /// Channel count.
    pub channels: u16,
}

/// Which hardware endpoints to open.
pub struct HardwareDeviceConfig {
    /// Optional capture device id.
    pub input: Option<AudioDeviceId>,
    /// Optional playback device id.
    pub output: Option<AudioDeviceId>,
}

/// Backend-specific device identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioDeviceId {
    /// Numeric host id (e.g. PipeWire node id).
    Numeric(u64),
    /// Textual host id (e.g. CPAL device name).
    Text(String),
}

/// Metadata for a discoverable audio endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioDeviceInfo {
    /// Device identifier used when opening.
    pub id: AudioDeviceId,
    /// Human-readable name.
    pub name: String,
    /// Capture or playback.
    pub direction: AudioDirection,
    /// Formats the device claims to support.
    pub formats: Vec<AudioFormat>,
    /// Preferred format when none is specified.
    pub default_format: AudioFormat,
    /// Whether this is the host default for its direction.
    pub default: bool,
    /// Whether the endpoint is a virtual/software device.
    pub virtual_device: bool,
}

/// Capture vs playback direction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioDirection {
    /// Microphone / capture.
    Input,
    /// Speaker / playback.
    Output,
}

/// Sample rate in Hz.
pub type SampleRate = u32;
