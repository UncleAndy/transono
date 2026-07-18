use anyhow::Result;

use libereco::audio::AudioDeviceFactory;
use libereco::audio::pipewire::device::PipeWireDeviceFactory;

fn main() -> Result<()> {
    let factory = PipeWireDeviceFactory;

    let devices = factory.enumerate_devices()?;

    for device in devices {
        println!("{:#?}", device);
    }

    Ok(())
}
