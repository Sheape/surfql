use surfql_storage::Storage;

use std::time::Duration;

use futures::stream::StreamExt;
use reqwest::{Client, ClientBuilder};

use crate::Result;

const USER_AGENT: &str = concat!(
    "SurfQL-Bot/v",
    env!("CARGO_PKG_VERSION"),
    " (contact: paulpare@protonmail.com)"
);

#[derive(Clone)]
pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub async fn new() -> Result<Self> {
        let client = ClientBuilder::new()
            .no_zstd()
            .no_gzip()
            .no_brotli()
            .pool_max_idle_per_host(20)
            .tcp_keepalive(Duration::from_secs(60))
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self { client })
    }

    pub async fn download_file_stream(&self, url: impl AsRef<str>, filename: &str) -> Result<()> {
        let storage = Storage::new()?;
        let container_client = storage.blob_container_client()?;

        let response = self.client.get(url.as_ref()).send().await?;
        let mut stream = response.bytes_stream();

        storage
            .upload_file_by_stream(container_client, filename, stream)
            .await?;

        Ok(())
    }
}
