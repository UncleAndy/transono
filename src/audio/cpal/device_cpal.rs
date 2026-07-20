use anyhow::{Context, Result};
use cpal::{traits::{DeviceTrait, HostTrait}, Device, Host, SampleFormat};
use crate::audio::{Endianness, PcmFormat};

/// Manager for audio devices discovered via CPAL.
pub struct AudioDevicesCpal {
    host: Host,
}

impl AudioDevicesCpal {
    /// Creates a new manager with the default host.
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
        }
    }

    /// Returns a reference to the CPAL host.
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// Returns the default input device.
    pub fn default_input(&self) -> Result<Device> {
        self.host
            .default_input_device()
            .context("Default input device not found")
    }

    /// Returns the default output device.
    pub fn default_output(&self) -> Result<Device> {
        self.host
            .default_output_device()
            .context("Default output device not found")
    }

    /// Returns a list of all available input devices.
    pub fn input_devices(&self) -> Result<Vec<Device>> {
        Ok(self.host.input_devices()?.collect())
    }

    /// Returns a list of all available output devices.
    pub fn output_devices(&self) -> Result<Vec<Device>> {
        Ok(self.host.output_devices()?.collect())
    }

    /// Finds an input device by its identifier.
    pub fn find_input(&self, id: &str) -> Result<Device> {
        let wanted = id.parse()?;

        self.host
            .device_by_id(&wanted)
            .context("Input device not found")
    }

    /// Finds an output device by its identifier.
    pub fn find_output(&self, id: &str) -> Result<Device> {
        let wanted = id.parse()?;

        self.host
            .device_by_id(&wanted)
            .context("Output device not found")
    }
}

#[allow(unused)]
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

#[allow(unused)]
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

pub(crate) fn sample_to_pcm_format(format: SampleFormat) -> PcmFormat {
    match format {
        SampleFormat::I8 => PcmFormat::I8,
        SampleFormat::I16 => PcmFormat::I16(Endianness::Little),
        SampleFormat::I24 => PcmFormat::I24(Endianness::Little),
        SampleFormat::I32 => PcmFormat::I32(Endianness::Little),
        SampleFormat::I64 => PcmFormat::I64(Endianness::Little),
        SampleFormat::U8 => PcmFormat::U8,
        SampleFormat::U16 => PcmFormat::U16(Endianness::Little),
        SampleFormat::U24 => PcmFormat::U24(Endianness::Little),
        SampleFormat::U32 => PcmFormat::U32(Endianness::Little),
        SampleFormat::U64 => PcmFormat::U64(Endianness::Little),
        SampleFormat::F32 => PcmFormat::F32(Endianness::Little),
        SampleFormat::F64 => PcmFormat::F64(Endianness::Little),
        SampleFormat::DsdU8 => PcmFormat::DsdU8,
        SampleFormat::DsdU16 => PcmFormat::DsdU16(Endianness::Little),
        SampleFormat::DsdU32 => PcmFormat::DsdU32(Endianness::Little),
        _ => PcmFormat::U8,
    }
}