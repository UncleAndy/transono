use crate::core::error::Result;
use crate::ctl::backend::Backend;

pub fn run(
    backend: &dyn Backend,
    language: &str,
) -> Result<()> {
    backend.remove(language)
}
