use crate::core::error::Result;
use crate::ctl::backend::Backend;

#[cfg(target_os = "linux")]
use crate::ctl::pipewire::PipewireBackend;

#[cfg(target_os = "windows")]
use crate::ctl::windows::WindowsBackend;

pub mod commands;
pub mod backend;

#[cfg(target_os = "linux")]
pub mod pipewire;

#[cfg(target_os = "windows")]
pub mod windows;

pub mod state;

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