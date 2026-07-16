use crate::audio::{AudioFormat, AudioInput, AudioOutput};
use crate::core::error::Result;

pub trait AudioDevice {}

#[allow(unused)]
pub struct AudioDevices {
    input: Vec<Box<dyn AudioInput>>,
    output: Vec<Box<dyn AudioOutput>>,
}

pub type DefaultAudioDeviceFactory = dyn AudioDeviceFactory;

#[allow(unused)]
pub trait AudioDeviceFactory {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>>;

    fn open_hardware(
        &self,
        config: &HardwareDeviceConfig,
    ) -> Result<AudioDevices>;

    fn create_virtual(
        &self,
        config: &VirtualDeviceConfig,
    ) -> Result<AudioDevices>;
}

pub struct VirtualDeviceConfig {
    pub name: String,
    pub sample_rate: SampleRate,
    pub channels: u16,
}

pub struct HardwareDeviceConfig {
    pub input: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioDeviceId {
    Numeric(u64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioDeviceInfo {
    pub id: AudioDeviceId,
    pub name: String,
    pub direction: AudioDirection,
    pub formats: Vec<AudioFormat>,
    pub default_format: AudioFormat,
    pub default: bool,
    pub virtual_device: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioDirection {
    Input,
    Output,
}

pub type SampleRate = u32;
