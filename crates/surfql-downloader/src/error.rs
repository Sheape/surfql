use reqwest::Error as ReqwestError;
use std::io::Error as IOError;
use surfql_queue::Error as QueueError;
use surfql_storage::Error as StorageError;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot initialize message queue: {0}")]
    MessageQueueInitFailed(#[from] QueueError),

    #[error("Storage failed: {0}")]
    StorageError(#[from] StorageError),

    #[error("Request failed: {0}")]
    Reqwest(#[from] ReqwestError),

    #[error("Invalid URL format: {url}")]
    InvalidURLFormat { url: String },

    #[error("Invalid file input: {filepath}: {source}")]
    InvalidFileInput {
        filepath: String,
        #[source]
        source: IOError,
    },
}
