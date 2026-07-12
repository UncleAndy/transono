use crate::core::error::Result;
use crate::ctl::backend::Backend;

pub fn run(
    backend: &dyn Backend,
) -> Result<()> {

    let status = backend.status()?;

    for item in status {
        println!("{item}");
    }

    Ok(())
}
