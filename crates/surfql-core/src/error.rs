use thiserror::Error;

pub(crate) type Result<T> = core::result::Result<T, Error>;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Missing configuration: {0}")]
    ConfigMissingEnv(&'static str),
}
