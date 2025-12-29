use std::time::Duration;

use crate::{Result, config::load_config};
use amqprs::{
    BasicProperties, DELIVERY_MODE_PERSISTENT,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, Channel, ConfirmSelectArguments, QueueDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
};
use azure_identity::DeveloperToolsCredential;
use azure_storage_blob::{
    BlobContainerClient, BlobContainerClientOptions, BlockBlobClient,
    models::{BlockBlobClientCommitBlockListOptions, BlockLookupList},
};
use futures::stream::StreamExt;
use reqwest::{Client, ClientBuilder};
use tokio::time::sleep;
use typespec::Bytes;

const USER_AGENT: &str = concat!(
    "SurfQL-Bot/v",
    env!("CARGO_PKG_VERSION"),
    " (contact: paulpare@protonmail.com)"
);
const BASE_URL: &str = "https://data.commoncrawl.org";
const MAX_SIZE_PER_CHUNK: usize = 8 * 1024 * 1024;
const QUEUE_NAME: &str = "warc_tasks";

pub struct Downloader {
    client: Client,
    rabbitmq_connection: Connection,
    rabbitmq_channel: Channel,
}

impl Downloader {
    pub async fn new() -> Result<Self> {
        let client = ClientBuilder::new()
            .no_zstd()
            .no_gzip()
            .no_brotli()
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .user_agent(USER_AGENT)
            .build()?;

        let args = OpenConnectionArguments::new("localhost", 5672, "user", "password");
        let rabbitmq_connection = Connection::open(&args).await?;
        rabbitmq_connection
            .register_callback(DefaultConnectionCallback)
            .await?;

        let rabbitmq_channel = rabbitmq_connection.open_channel(None).await?;
        rabbitmq_channel
            .register_callback(DefaultChannelCallback)
            .await?;

        let queue_arg = QueueDeclareArguments::new(QUEUE_NAME)
            .durable(true)
            .finish();
        rabbitmq_channel.queue_declare(queue_arg).await?;
        rabbitmq_channel
            .confirm_select(ConfirmSelectArguments::new(true))
            .await?;

        Ok(Self {
            client,
            rabbitmq_connection,
            rabbitmq_channel,
        })
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

    pub async fn download_file(&self, url: impl AsRef<str>) -> Result<()> {
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
            let mut buffer = Vec::with_capacity(MAX_SIZE_PER_CHUNK);

            while let Some(chunk) = stream.next().await {
                let chunk_res = chunk?;
                buffer.extend_from_slice(&chunk_res);

                if buffer.len() >= MAX_SIZE_PER_CHUNK {
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

    pub async fn close_rabbitmq_channel(self) -> Result<()> {
        Ok(self.rabbitmq_channel.close().await?)
    }

    pub async fn close_rabbitmq_connection(self) -> Result<()> {
        sleep(Duration::from_secs(2)).await;
        Ok(self.rabbitmq_connection.close().await?)
    }

    pub async fn publish_paths(&self) -> Result<()> {
        let warc_paths = include_str!("../../../samples/warc.paths");
        for (index, path) in warc_paths.lines().enumerate() {
            let props = BasicProperties::default()
                .with_delivery_mode(DELIVERY_MODE_PERSISTENT)
                .with_content_type("text/plain")
                .finish();
            let content = Vec::from(path.as_bytes());
            let publish_args = BasicPublishArguments::new("", QUEUE_NAME);

            self.rabbitmq_channel
                .basic_publish(props, content, publish_args)
                .await?;
            if 5000 % (index + 1) == 0 {
                println!(
                    "Progress: {index}/100,000 ({:.2}%)",
                    (index as f32 / 100_000_f32) * 100.0
                )
            }
        }

        Ok(())
    }
}
