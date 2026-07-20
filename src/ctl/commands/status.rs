use crate::core::error::Result;
use crate::ctl::backend::Backend;

/// Executes the status command.
pub fn run(
    backend: &dyn Backend,
    lang: &str,
) -> Result<()> {

    let status = backend.status(lang)?;

    for item in status {
        println!("{item}");
    }

    Ok(())
}
