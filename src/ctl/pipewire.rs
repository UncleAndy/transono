use std::num::ParseIntError;
use std::process::{Command, Output};
use std::string::FromUtf8Error;
use anyhow::anyhow;
use crate::core::error::{CoreError, Result};
use crate::ctl::backend::{Backend, DeviceSet, DeviceStatus, DoctorReport};

pub struct PipewireBackend;

impl PipewireBackend {
    pub fn new() -> Result<PipewireBackend> {
        Ok(Self{})
    }
}

impl Backend for PipewireBackend {
    fn init(&self, lang: &str) -> Result<()> {
        let devices = VirtualAudioDevices::create(lang)?;

        // "Забываем" о виртуальных устройствах, т.к. они должны сохраниться
        // после выхода из приложения.
        std::mem::forget(devices);

        Ok(())
    }

    fn remove(&self, lang: &str) -> Result<()> {
        VirtualAudioDevices::cleanup(Some(lang))
    }

    fn devices(&self, lang: &str) -> Result<DeviceSet> {
        let devices = VirtualAudioDevices::create(lang)?;

        let set = devices.device_set();

        std::mem::forget(devices);

        Ok(set)
    }

    fn status(&self) -> Result<Vec<DeviceStatus>> {
        todo!()
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

    sink_module: u32,
    source_module: u32,
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

    pub fn cleanup(lang: Option<&str>) -> Result<()> {
        let output = Command::new("pactl")
            .args(["list", "short", "modules"])
            .output();

        if !output.is_ok() {
            return Err(CoreError::Other(anyhow!("failed to execute 'pactl list short modules'")));
        };

        let stdout = String::from_utf8(output.unwrap().stdout);
        let stdout = match stdout {
            Ok(stdout) => stdout,
            Err(e) => return Err(CoreError::Other(anyhow!(e)))
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
                .map_err(|e| {
                    CoreError::Other(anyhow!(e))
                })?;

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
    let sink_module = load_module(
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
    let source_module = load_module(
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
        sink_module,
        source_module,
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
        .map_err(|e| { CoreError::Other(anyhow!(e)) })?;

    if !out.status.success() {
        return Err(CoreError::Other(anyhow!("{}", String::from_utf8_lossy(&out.stderr))));
    }

    let res = String::from_utf8(out.stdout)
        .map_err(|e| CoreError::Other(anyhow!(e)))?
        .trim().parse()
        .map_err(|e: ParseIntError| CoreError::Other(anyhow!(e)))?;

    Ok(res)
}

fn unload_module(id: u32) -> Result<()> {
    let status = Command::new("pactl")
        .args(["unload-module", &id.to_string()])
        .status()
        .map_err(|e| CoreError::Other(anyhow!(e)))?;

    if !status.success() {
        return Err(CoreError::Other(anyhow!("pactl unload-module failed")));
    }

    Ok(())
}
