use cpal::traits::DeviceTrait;
use libereco::audio::AudioDevicesCpal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let devices = AudioDevicesCpal::new();

    println!("Output devices:");
    let out_devs = devices.output_devices()?;
    for dev in out_devs.clone() {
        println!("    {} {:?} {:?}", dev.to_string(), dev.default_input_config(), dev.default_output_config())
    }

    println!("Input devices:");
    let in_devs = devices.input_devices()?;
    for dev in in_devs.clone() {
        println!("    {} {:?} {:?}", dev.to_string(), dev.default_input_config(), dev.default_output_config())
    }

    Ok(())
}