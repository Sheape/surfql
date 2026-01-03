mod error;
mod storage;
mod utils;

pub(crate) use utils::MAX_SIZE_PER_CHUNK;
pub use error::{Error, Result};
pub use storage::Storage;
