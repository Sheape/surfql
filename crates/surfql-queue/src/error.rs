use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Message queue error: {0}")]
    Amqprs(#[from] amqprs::error::Error),
}
