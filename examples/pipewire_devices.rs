use anyhow::Result;

use transono::audio::AudioDeviceFactory;
use transono::audio::pipewire::device::PipeWireDeviceFactory;

fn main() -> Result<()> {
    let factory = PipeWireDeviceFactory;

    let devices = factory.enumerate_devices()?;

    for device in devices {
        println!("{:#?}", device);
    }

    Ok(())
}
