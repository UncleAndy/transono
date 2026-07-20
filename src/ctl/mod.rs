use crate::core::error::Result;
use crate::ctl::backend::Backend;

#[cfg(target_os = "linux")]
use crate::ctl::pipewire::PipewireBackend;

#[cfg(target_os = "windows")]
use crate::ctl::windows::WindowsBackend;

/// CLI commands for device management.
pub mod commands;
/// Generic backend traits and types.
pub mod backend;

#[cfg(target_os = "linux")]
/// Linux-specific PipeWire backend.
pub mod pipewire;

#[cfg(target_os = "windows")]
/// Windows-specific backend.
pub mod windows;

/// State management for virtual devices.
pub mod state;

/// Creates a backend instance suitable for the current operating system.
pub fn create_backend() -> Result<Box<dyn Backend>> {
    #[cfg(target_os = "linux")]
    {
        let backend_res = PipewireBackend::new();
        match backend_res {
            Ok(backend) => Ok(Box::new(backend)),
            Err(e) => Err(e),
        }
    }

    #[cfg(target_os = "windows")]
    {
        let backend_res = WindowsBackend::new();
        match backend_res {
            Ok(backend) => Ok(Box::new(backend)),
            Err(e) => Err(e),
        }
    }
}