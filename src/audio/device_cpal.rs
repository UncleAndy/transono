use anyhow::{Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Host,
};

pub struct AudioDevicesCpal {
    host: Host,
}

impl AudioDevicesCpal {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    pub fn host(&self) -> &Host {
        &self.host
    }

    pub fn default_input(&self) -> Result<Device> {
        self.host
            .default_input_device()
            .context("Default input device not found")
    }

    pub fn default_output(&self) -> Result<Device> {
        self.host
            .default_output_device()
            .context("Default output device not found")
    }

    pub fn input_devices(&self) -> Result<Vec<Device>> {
        Ok(self.host.input_devices()?.collect())
    }

    pub fn output_devices(&self) -> Result<Vec<Device>> {
        Ok(self.host.output_devices()?.collect())
    }

    pub fn find_input(&self, id: &str) -> Result<Device> {
        let wanted = id.parse()?;

        self.host
            .device_by_id(&wanted)
            .context("Input device not found")
    }

    pub fn find_output(&self, id: &str) -> Result<Device> {
        let wanted = id.parse()?;

        self.host
            .device_by_id(&wanted)
            .context("Output device not found")
    }
}

fn print_device(device: &Device) -> Result<()> {
    println!("Device: {}", device.id()?);

    let description = device.description()?;

    println!("{description:#?}");

    if device.supports_input() {
        println!("INPUT CONFIGS:");

        for cfg in device.supported_input_configs()? {
            print_config(cfg);
        }
    }

    if device.supports_output() {
        println!("OUTPUT CONFIGS:");

        for cfg in device.supported_output_configs()? {
            print_config(cfg);
        }
    }

    Ok(())
}

fn print_config(cfg: cpal::SupportedStreamConfigRange) {
    println!(
        "  {} ch | {:?} | {}..{} Hz",
        cfg.channels(),
        cfg.sample_format(),
        cfg.min_sample_rate(),
        cfg.max_sample_rate()
    );

    println!("    {:?}", cfg.buffer_size());
}
