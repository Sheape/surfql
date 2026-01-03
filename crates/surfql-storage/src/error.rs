use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Request while streaming failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Azure Blob storage error: {0}")]
    Typespec(#[from] typespec::Error),
}
