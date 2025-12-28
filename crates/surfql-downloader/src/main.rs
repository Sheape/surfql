mod config;
mod downloader;
mod error;

pub use error::{Error, Result};

use crate::downloader::Downloader;

#[tokio::main]
async fn main() -> Result<()> {
    Downloader::new()?.download("crawl-data/CC-MAIN-2025-51/segments/1764871306713.64/warc/CC-MAIN-20251204191828-20251204221828-00001.warc.gz").await?;
    Ok(())
}
