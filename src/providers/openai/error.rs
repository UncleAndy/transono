use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenAiError {

    #[error(transparent)]
    Other(
        #[from]
        anyhow::Error,
    ),

}
