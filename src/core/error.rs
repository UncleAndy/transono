use thiserror::Error;

pub type Result<T> =
std::result::Result<T, CoreError>;

#[derive(Debug, Error)]
pub enum CoreError {

    #[error(transparent)]
    Other(
        #[from]
        anyhow::Error,
    ),

}
