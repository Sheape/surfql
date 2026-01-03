mod config;
mod error;

pub use config::load_config;
pub(crate) use error::{Error, Result};
