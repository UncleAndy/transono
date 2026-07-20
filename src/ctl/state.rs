use std::path::PathBuf;

use crate::core::error::Result;

/// Persistent state for device management.
pub struct State;

impl State {
    /// Creates a new state manager.
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Returns the path to the configuration directory.
    pub fn config_dir(&self) -> Result<PathBuf> {
        todo!()
    }

    /// Returns the path to the state directory.
    pub fn state_dir(&self) -> Result<PathBuf> {
        todo!()
    }
}