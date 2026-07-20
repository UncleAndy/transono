use crate::core::error::Result;
use crate::ctl::backend::Backend;

/// Executes the repair command.
pub fn run(
    backend: &dyn Backend,
    language: &str,
) -> Result<()> {
    backend.remove(language)?;

    backend.init(language)
}
