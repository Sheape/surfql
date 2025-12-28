use std::time::Duration;

use crate::{Result, config::load_config};
use azure_identity::DeveloperToolsCredential;
use azure_storage_blob::{
    BlobContainerClient, BlobContainerClientOptions, BlockBlobClient,
    models::{BlockBlobClientCommitBlockListOptions, BlockLookupList},
};
use futures::stream::StreamExt;
use reqwest::{Client, ClientBuilder};
use typespec::Bytes;

const USER_AGENT: &str = concat!(
    "SurfQL-Bot/v",
    env!("CARGO_PKG_VERSION"),
    " (contact: paulpare@protonmail.com)"
);
const BASE_URL: &str = "https://data.commoncrawl.org";

#[derive(Debug)]
pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        let client = ClientBuilder::new()
            .no_zstd()
            .no_gzip()
            .no_brotli()
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .user_agent(USER_AGENT)
            .build()?;

        Ok(Self { client })
    }

    fn get_container_client(&self) -> Result<BlobContainerClient> {
        let config = load_config();
        let credential = DeveloperToolsCredential::new(None)?;

        Ok(BlobContainerClient::new(
            &config.STORAGE_ACCOUNT_ENDPOINT,
            &config.STORAGE_CONTAINER,
            Some(credential),
            Some(BlobContainerClientOptions::default()),
        )?)
    }

    async fn upload_block(
        &self,
        block_blob_client: &BlockBlobClient,
        buffer: &mut Vec<u8>,
        index: &mut u32,
    ) -> Result<Vec<u8>> {
        let block_id_raw = format!("{index:06}");
        let block_id = azure_core::base64::encode(&block_id_raw).into_bytes();
        println!("Uploading chunk {block_id_raw}...");

        let chunk = Bytes::copy_from_slice(buffer);
        block_blob_client
            .stage_block(&block_id, chunk.len() as u64, chunk.into(), None)
            .await?;

        buffer.clear();
        *index += 1;

        Ok(block_id)
    }

    pub async fn download(self, url: impl AsRef<str>) -> Result<()> {
        println!("Downloading {BASE_URL}/{}...", url.as_ref());
        let response = self
            .client
            .get(format!("{BASE_URL}/{}", url.as_ref()))
            .send()
            .await?;
        let mut stream = response.bytes_stream();
        let container_client = self.get_container_client()?;
        if let Some((_, filename)) = url.as_ref().rsplit_once('/') {
            let blob_client = container_client.blob_client(filename);
            let block_blob_client = blob_client.block_blob_client();

            let mut block_ids: Vec<Vec<u8>> = vec![];
            let mut chunk_index = 0_u32;
            let mut buffer = Vec::with_capacity(8 * 1024 * 1024);

            while let Some(chunk) = stream.next().await {
                let chunk_res = chunk?;
                buffer.extend_from_slice(&chunk_res);

                if buffer.len() >= 8 * 1024 * 1024 {
                    let chunk_id = self
                        .upload_block(&block_blob_client, &mut buffer, &mut chunk_index)
                        .await?;
                    block_ids.push(chunk_id);
                }
            }

            if !buffer.is_empty() {
                let chunk_id = self
                    .upload_block(&block_blob_client, &mut buffer, &mut chunk_index)
                    .await?;
                block_ids.push(chunk_id);
            }

            let block_lookup_list = BlockLookupList {
                committed: Some(vec![]),
                latest: Some(block_ids),
                uncommitted: Some(vec![]),
            };

            let commit_options = BlockBlobClientCommitBlockListOptions {
                blob_content_type: Some("application/warc".into()),
                blob_content_encoding: Some("gzip".into()),
                ..Default::default()
            };

            println!("Uploading all blocks");
            block_blob_client
                .commit_block_list(block_lookup_list.try_into()?, Some(commit_options))
                .await?;
            println!("Uploaded all blocks from {BASE_URL}/{}", url.as_ref());
        }

        Ok(())
    }
}
