use crate::audio::{AudioDeviceFactory, AudioDeviceInfo, AudioDevices, HardwareDeviceConfig, VirtualDeviceConfig};

pub struct PipeWireDeviceFactory;

impl AudioDeviceFactory for PipeWireDeviceFactory {
    fn enumerate_devices(&self) -> crate::core::error::Result<Vec<AudioDeviceInfo>> {
        todo!()
    }

    fn open_hardware(&self, config: &HardwareDeviceConfig) -> crate::core::error::Result<AudioDevices> {
        todo!()
    }

    fn create_virtual(&self, config: &VirtualDeviceConfig) -> crate::core::error::Result<AudioDevices> {
        todo!()
    }
}
