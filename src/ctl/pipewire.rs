use std::num::ParseIntError;
use std::process::Command;
use cpal::traits::HostTrait;

use crate::core::error::{CoreError, Result};
use crate::ctl::backend::{Backend, DeviceSet, DeviceState, DeviceStatus, DoctorReport};

pub struct PipewireBackend;

impl PipewireBackend {
    pub fn new() -> Result<PipewireBackend> {
        Ok(Self{})
    }
}

impl Backend for PipewireBackend {
    fn init(&self, lang: &str) -> Result<()> {
        let status = self.status(lang)?;

        let present = status
            .iter()
            .filter(|s| matches!(s.state, DeviceState::Present))
            .count();

        match present {
            4 => return Ok(()),

            0 => {
                let devices = VirtualAudioDevices::create(lang)?;
                std::mem::forget(devices);
                Ok(())
            }

            _ => {
                VirtualAudioDevices::cleanup(Some(lang))?;

                let devices = VirtualAudioDevices::create(lang)?;
                std::mem::forget(devices);

                Ok(())
            }
        }
    }

    fn remove(&self, lang: &str) -> Result<()> {
        VirtualAudioDevices::cleanup(Some(lang))
    }

    fn devices(&self, lang: &str) -> Result<DeviceSet> {
        Ok(VirtualAudioDevices::names(lang))
    }

    fn status(&self, lang: &str) -> Result<Vec<DeviceStatus>> {
        let host = cpal::default_host();

        let inputs = host
            .input_devices()
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .collect::<Vec<_>>();

        let outputs = host
            .output_devices()
            .map_err(|e| CoreError::Internal(e.to_string()))?
            .collect::<Vec<_>>();

        let mut result = Vec::new();

        // Пока проверяем только один язык.
        // Позже это можно будет заменить на чтение State.
        let devices = VirtualAudioDevices::names(lang);

        result.push(DeviceStatus {
            name: devices.to_meeting_microphone.clone(),
            state: if has_device(inputs.iter().cloned(), &devices.to_meeting_microphone) {
                DeviceState::Present
            } else {
                DeviceState::Missing
            },
        });

        result.push(DeviceStatus {
            name: devices.from_meeting_speaker.clone(),
            state: if has_device(outputs.iter().cloned(), &devices.from_meeting_speaker) {
                DeviceState::Present
            } else {
                DeviceState::Missing
            },
        });

        result.push(DeviceStatus {
            name: devices.internal_from_meeting_microphone.clone(),
            state: if has_device(inputs.iter().cloned(), &devices.internal_from_meeting_microphone) {
                DeviceState::Present
            } else {
                DeviceState::Missing
            },
        });

        result.push(DeviceStatus {
            name: devices.internal_to_meeting_speaker.clone(),
            state: if has_device(outputs.iter().cloned(), &devices.internal_to_meeting_speaker) {
                DeviceState::Present
            } else {
                DeviceState::Missing
            },
        });

        Ok(result)
    }

    fn doctor(&self) -> Result<DoctorReport> {
        todo!()
    }
}

#[derive(Clone)]
pub struct VirtualAudioDevices {
    pub from: VirtualAudioDevicePair,
    pub to: VirtualAudioDevicePair,
}

#[derive(Clone)]
pub struct VirtualAudioDevicePair {
    pub sink_name: String,
    pub source_name: String,

    pub output_name: String,
    pub input_name: String,
}

impl VirtualAudioDevices {
    /// Возвращает набор:
    /// - Объект VirtualDevices для сохранения его времени жизни
    /// - Строку с именем устройства воспроизведения для передачи аудио в микрофон
    /// - Строку с именем устройства чтения для чтения аудиоданных со встречи
    pub fn create(lang: &str) -> Result<Self> {
        let to_pair = create_pair(lang, "ToMeeting", HiddenDevice::Output)?;
        let from_pair = create_pair(lang, "FromMeeting", HiddenDevice::Input)?;
        Ok(
            Self {
                to: to_pair.clone(),
                from: from_pair.clone(),
            }
        )
    }

    pub fn names(lang: &str) -> DeviceSet {
        let lang = lang.to_uppercase();

        DeviceSet {
            to_meeting_microphone:
            format!("Translator.{lang}.ToMeeting.Microphone"),

            from_meeting_speaker:
            format!("Translator.{lang}.FromMeeting.Speaker"),

            internal_to_meeting_speaker:
            format!("___internal.not_use.{lang}"),

            internal_from_meeting_microphone:
            format!("___internal.not_use.{lang}_"),
        }
    }

    pub fn cleanup(lang: Option<&str>) -> Result<()> {
        let output = Command::new("pactl")
            .args(["list", "short", "modules"])
            .output();

        if !output.is_ok() {
            return Err(CoreError::Internal("failed to execute 'pactl list short modules'".to_string()));
        };

        let stdout = String::from_utf8(output.unwrap().stdout);
        let stdout = match stdout {
            Ok(stdout) => stdout,
            Err(e) => return Err(CoreError::Internal(e.to_string()))
        };

        // Сначала собираем id, потом удаляем.
        // Это безопаснее, если список модулей во время удаления изменится.
        let mut modules = Vec::<u32>::new();

        let prefix;
        let public;

        match lang {
            Some(lang) => {
                prefix = format!("translator_{lang}");
                public = format!("Translator.{}", lang.to_uppercase());
            }
            None => {
                prefix = "translator_".to_owned();
                public = "Translator.".to_owned();
            }
        }

        for line in stdout.lines() {
            if !(line.contains(&prefix) || line.contains(&public)) {
                continue;
            }

            let Some(id) = line.split_whitespace().next() else {
                continue;
            };

            if let Ok(id) = id.parse::<u32>() {
                modules.push(id);
            }
        }

        // Лучше удалять в обратном порядке:
        // сначала remap-source, потом null-sink.
        modules.sort_unstable_by(|a, b| b.cmp(a));

        for id in modules {
            let status = Command::new("pactl")
                .args(["unload-module", &id.to_string()])
                .status()
                .map_err(|e| CoreError::Internal(e.to_string()))?;

            if !status.success() {
                eprintln!("Failed to unload PulseAudio module {}", id);
            }
        }

        Ok(())
    }

    pub fn device_set(&self) -> DeviceSet {
        DeviceSet {
            to_meeting_microphone:
            self.to.input_name.clone(),

            from_meeting_speaker:
            self.from.output_name.clone(),

            internal_to_meeting_speaker:
            self.to.output_name.clone(),

            internal_from_meeting_microphone:
            self.from.input_name.clone(),
        }
    }
}

enum HiddenDevice {
    Input,
    Output,
}

fn create_pair(lang: &str, prefix: &str, hide_device: HiddenDevice) -> Result<VirtualAudioDevicePair> {
    let lang = lang.to_lowercase();

    let sink_name = format!("translator_{lang}_{prefix}_speaker");
    let source_name = format!("translator_{lang}_{prefix}_microphone");

    let output_name = match hide_device {
        HiddenDevice::Output => {
            format!("___internal.not_use.{}", lang.to_uppercase())
        }
        HiddenDevice::Input => {
            format!("Translator.{}.{}.Speaker", lang.to_uppercase(), prefix)
        }
    };
    let sink_properties = &format!("device.description={output_name}");
    let _ = load_module(
        "module-null-sink",
        &[
            ("sink_name", &sink_name),
            (
                "sink_properties",
                sink_properties,
            ),
        ],
    )?;

    let input_name = match hide_device {
        HiddenDevice::Output => {
            format!("Translator.{}.{}.Microphone", lang.to_uppercase(), prefix)
        }
        HiddenDevice::Input => {
            format!("___internal.not_use.{}_", lang.to_uppercase())
        }
    };
    let sink_properties = &format!("device.description={input_name}");
    let _ = load_module(
        "module-remap-source",
        &[
            ("master", &format!("{sink_name}.monitor")),
            ("source_name", &source_name),
            (
                "source_properties",
                sink_properties,
            ),
        ],
    )?;

    Ok(VirtualAudioDevicePair {
        sink_name,
        source_name,
        output_name,
        input_name,
    })
}

fn load_module(module: &str, args: &[(&str, &str)]) -> Result<u32> {
    let mut cmd = Command::new("pactl");

    cmd.arg("load-module");
    cmd.arg(module);

    for (k, v) in args {
        cmd.arg(format!("{k}={v}"));
    }

    let out = cmd.output()
        .map_err(|e| CoreError::Internal(e.to_string()))?;

    if !out.status.success() {
        return Err(CoreError::Internal(format!("{}", String::from_utf8_lossy(&out.stderr))));
    }

    let res = String::from_utf8(out.stdout)
        .map_err(|e| CoreError::Internal(e.to_string()))?
        .trim().parse()
        .map_err(|e: ParseIntError| CoreError::Internal(e.to_string()))?;

    Ok(res)
}

#[allow(unused)]
fn unload_module(id: u32) -> Result<()> {
    let status = Command::new("pactl")
        .args(["unload-module", &id.to_string()])
        .status()
        .map_err(|e| CoreError::Internal(e.to_string()))?;

    if !status.success() {
        return Err(CoreError::Internal("pactl unload-module failed".to_string()));
    }

    Ok(())
}

fn has_device<I>(
    devices: I,
    name: &str,
) -> bool
where
    I: IntoIterator<Item = cpal::Device>,
{
    devices
        .into_iter()
        .filter_map(|d|
            Some(d.to_string())
        )
        .any(|n| n == name)
}
