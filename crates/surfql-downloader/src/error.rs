use derive_more::From;
use serde::Serialize;
use serde_with::{DisplayFromStr, serde_as};

pub type Result<T> = core::result::Result<T, Error>;

#[serde_as]
#[derive(Debug, From, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Error {
    ConfigMissingEnv(&'static str),

    #[from]
    Reqwest(#[serde_as(as = "DisplayFromStr")] reqwest::Error),

    #[from]
    Typespec(#[serde_as(as = "DisplayFromStr")] typespec::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
