use crate::core::error::Result;
use crate::ctl::backend::Backend;

pub fn run(
    backend: &dyn Backend,
) -> Result<()> {

    let report = backend.doctor()?;

    println!("{report}");

    Ok(())
}
