use crate::core::error::Result;
use crate::ctl::backend::{Backend, DeviceInfo, DeviceStatus, DoctorReport};

pub struct PipewireBackend;

impl PipewireBackend {
    pub fn new() -> Result<PipewireBackend> {
        todo!()
    }
}

impl Backend for PipewireBackend {
    fn init(&self, lang: &str) -> Result<()> {
        todo!()
    }

    fn remove(&self, lang: &str) -> Result<()> {
        todo!()
    }

    fn list(&self) -> Result<Vec<DeviceInfo>> {
        todo!()
    }

    fn status(&self) -> Result<Vec<DeviceStatus>> {
        todo!()
    }

    fn doctor(&self) -> Result<DoctorReport> {
        todo!()
    }
}
