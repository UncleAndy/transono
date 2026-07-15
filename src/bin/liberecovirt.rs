use anyhow::{anyhow, Result};
use clap::Parser;
use pipewire::main_loop::MainLoopRc;
use pipewire::context::ContextRc;
use std::ffi::CString;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::io::{Read, Write};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Language code or 'remove' command
    action: String,
    /// Language code if first argument is 'remove'
    language: Option<String>,
}

fn get_pid_file(language: &str) -> PathBuf {
    let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    path.push("liberecovirt");
    let _ = fs::create_dir_all(&path);
    path.push(format!("{}.pid", language));
    path
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (language, is_remove) = if cli.action == "remove" {
        if let Some(lang) = cli.language {
            (lang, true)
        } else {
            return Err(anyhow!("Укажите язык для удаления: liberecovirt remove <lang>"));
        }
    } else {
        (cli.action, false)
    };

    if is_remove {
        let pid_file = get_pid_file(&language);
        if !pid_file.exists() {
            println!("Устройства для языка {} не найдены", language);
            return Ok(());
        }

        let mut file = fs::File::open(&pid_file)?;
        let mut pid_str = String::new();
        file.read_to_string(&mut pid_str)?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Kill process
            let _ = process::Command::new("kill").arg(pid.to_string()).status();
            println!("Остановка устройств для языка {} (PID: {})", language, pid);
        }
        let _ = fs::remove_file(&pid_file);
        return Ok(());
    }

    // Start logic
    let pid_file = get_pid_file(&language);
    if pid_file.exists() {
        let mut file = fs::File::open(&pid_file)?;
        let mut pid_str = String::new();
        file.read_to_string(&mut pid_str)?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process exists
            if let Ok(process) = process::Command::new("ps").arg("-p").arg(pid.to_string()).output() {
                if process.status.success() {
                    println!("Виртуальные устройства для языка {} уже запущены (PID: {})", language, pid);
                    return Ok(());
                }
            }
        }
        // Stale pid file
        let _ = fs::remove_file(&pid_file);
    }

            // Start logic continued...
            // Start as daemon
            let pid = process::id();
            let mut file = fs::File::create(&pid_file)?;
            file.write_all(pid.to_string().as_bytes())?;

            println!("Запуск виртуальных устройств для языка: {}", language);

            pipewire::init();
            let main_loop = MainLoopRc::new(None).map_err(|e| anyhow!("Не удалось создать Main Loop: {}", e))?;
            let context = ContextRc::new(&main_loop, None).map_err(|e| anyhow!("Не удалось создать Context: {}", e))?;
            let _core = context.connect_rc(None).map_err(|e| anyhow!("Не удалось подключиться к Core: {}", e))?;

            let dev1_args = format!("\
                capture.props = {{ \
                    node.name = \"user_speaker_{}\" \
                    node.description = \"Виртуальные Динамики Приложения ({})\" \
                    media.class = \"Audio/Sink\" \
                }} \
                playback.props = {{ \
                    node.name = \"app_capture_hidden_{}\" \
                    node.passive = true \
                    node.dont-reconnect = true \
                    node.always-process = true \
                }}", language, language, language);

            let dev2_args = format!("\
                capture.props = {{ \
                    node.name = \"app_playback_hidden_{}\" \
                    node.passive = true \
                    node.dont-reconnect = true \
                    node.always-process = true \
                }} \
                playback.props = {{ \
                    node.name = \"user_microphone_{}\" \
                    node.description = \"Виртуальный Микрофон Приложения ({})\" \
                    media.class = \"Audio/Source\" \
                }}", language, language, language);

            let _dev1_module = unsafe {
                let name = CString::new("libpipewire-module-loopback").unwrap();
                let args = CString::new(dev1_args).unwrap();
                pipewire::sys::pw_context_load_module(
                    context.as_raw_ptr(),
                    name.as_ptr(),
                    args.as_ptr(),
                    std::ptr::null_mut(),
                )
            };
            if _dev1_module.is_null() {
                return Err(anyhow!("Не удалось создать Устройство 1"));
            }

            let _dev2_module = unsafe {
                let name = CString::new("libpipewire-module-loopback").unwrap();
                let args = CString::new(dev2_args).unwrap();
                pipewire::sys::pw_context_load_module(
                    context.as_raw_ptr(),
                    name.as_ptr(),
                    args.as_ptr(),
                    std::ptr::null_mut(),
                )
            };
            if _dev2_module.is_null() {
                return Err(anyhow!("Не удалось создать Устройство 2"));
            }

            println!("Виртуальные кабели запущены для {}. PID: {}", language, pid);
            
            // Running main loop will block until process exits
            main_loop.run();

            // Cleanup on exit
            let _ = fs::remove_file(&pid_file);
            Ok(())
}
