use crate::core::error::Result;
use crate::ctl::backend::Backend;

/// Executes the init command.
pub fn run(
    backend: &dyn Backend,
    lang: &str,
) -> Result<()> {
    backend.init(lang)
}
