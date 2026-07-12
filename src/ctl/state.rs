use std::path::PathBuf;

use crate::core::error::Result;

pub struct State;

impl State {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn config_dir(&self) -> Result<PathBuf> {
        todo!()
    }

    pub fn state_dir(&self) -> Result<PathBuf> {
        todo!()
    }
}