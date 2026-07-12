use crate::core::error::Result;
use crate::ctl::backend::Backend;

pub fn run(
    backend: &dyn Backend,
) -> Result<()> {
    let devices = backend.list()?;

    for device in devices {
        println!("{device}");
    }

    Ok(())
}
