use anyhow::{Result, anyhow};
use std::process::Command;

/*
   Создания устройства воспроизведения (Виртуального динамика):
   $ pactl load-module module-null-sink
       sink_name=translator_out
       sink_properties=device.description="Translator.EN.Speaker"

   Создание виртуального микрофона:
   $ pactl load-module module-remap-source
       master=translator_out.monitor
       source_name=translator_in
       source_properties=device.description="Translator.EN.Microphone"

   Оба метода возвращают числовые идентификаторы, которые можно использовать для их удаления:
   $ pactl unload-module <num id>
*/

#[derive(Clone)]
pub struct VirtualDevices {
    pub from: VirtualDevicePair,
    pub to: VirtualDevicePair,
}

#[derive(Clone)]
pub struct VirtualDevicePair {
    pub sink_name: String,
    pub source_name: String,

    pub output_name: String,
    pub input_name: String,

    sink_module: u32,
    source_module: u32,
}

impl VirtualDevices {
    /// Возвращает набор:
    /// - Объект VirtualDevices для сохранения его времени жизни
    /// - Строку с именем устройства воспроизведения для передачи аудио в микрофон
    /// - Строку с именем устройства чтения для чтения аудиоданных со встречи
    pub fn create(lang: &str) -> Result<(Self, String, String)> {
        let to_pair = create_pair(lang, "ToMeeting", HiddenDevice::Output)?;
        let from_pair = create_pair(lang, "FromMeeting", HiddenDevice::Input)?;
        Ok((
            Self {
                to: to_pair.clone(),
                from: from_pair.clone(),
            },
            to_pair.output_name,
            from_pair.input_name,
        ))
    }

    pub fn cleanup() -> Result<()> {
        let output = Command::new("pactl")
            .args(["list", "short", "modules"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("failed to execute 'pactl list short modules'"));
        }

        let stdout = String::from_utf8(output.stdout)?;

        // Сначала собираем id, потом удаляем.
        // Это безопаснее, если список модулей во время удаления изменится.
        let mut modules = Vec::<u32>::new();

        for line in stdout.lines() {
            if !(line.contains("translator_") || line.contains("Translator.")) {
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
                .status()?;

            if !status.success() {
                eprintln!("Failed to unload PulseAudio module {}", id);
            }
        }

        Ok(())
    }
}

enum HiddenDevice {
    Input,
    Output,
}

fn create_pair(lang: &str, prefix: &str, hide_device: HiddenDevice) -> Result<VirtualDevicePair> {
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

    Ok(VirtualDevicePair {
        sink_name,
        source_name,
        output_name,
        input_name,
        sink_module,
        source_module,
    })
}

impl Drop for VirtualDevices {
    fn drop(&mut self) {
        println!("Unload modules for virtual devices...");
        let _ = unload_module(self.from.source_module);
        let _ = unload_module(self.from.sink_module);
        let _ = unload_module(self.to.source_module);
        let _ = unload_module(self.to.sink_module);
        println!("Unload modules done.");
    }
}

fn load_module(module: &str, args: &[(&str, &str)]) -> Result<u32> {
    let mut cmd = Command::new("pactl");

    cmd.arg("load-module");
    cmd.arg(module);

    for (k, v) in args {
        cmd.arg(format!("{k}={v}"));
    }

    let out = cmd.output()?;

    if !out.status.success() {
        return Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr)));
    }

    Ok(String::from_utf8(out.stdout)?.trim().parse()?)
}

fn unload_module(id: u32) -> Result<()> {
    let status = Command::new("pactl")
        .args(["unload-module", &id.to_string()])
        .status()?;

    if !status.success() {
        return Err(anyhow!("pactl unload-module failed"));
    }

    Ok(())
}
