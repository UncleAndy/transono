use crate::core::error::Result;
use crate::ctl::backend::Backend;

pub fn run(
    backend: &dyn Backend,
    lang: &str,
) -> Result<()> {
    let devices = backend.devices(lang)?;

    println!("{devices}");

    Ok(())
}
