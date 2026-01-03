use crate::{MAX_SIZE_PER_CHUNK, Result};
use surfql_core::load_config;
use surfql_telemetry::KeyValue;
#[cfg(feature = "telemetry")]
use surfql_telemetry::{CC_BYTES_UPLOADED_COUNTER, CC_FILES_UPLOADED_COUNTER};

use azure_core::{
    base64::encode,
    credentials::{Secret, TokenCredential},
};
use azure_identity::ClientSecretCredential;
use azure_storage_blob::models::BlockBlobClientCommitBlockListOptions;
use azure_storage_blob::models::BlockLookupList;
use azure_storage_blob::{BlobContainerClient, BlobContainerClientOptions, BlockBlobClient};
use futures::{Stream, StreamExt};
use std::{error::Error, sync::Arc};
use typespec::Bytes;

pub struct Storage {
    endpoint: String,
    container_name: String,
    credential: Arc<dyn TokenCredential>,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let config = load_config();
        let credential = ClientSecretCredential::new(
            &config.AZURE_TENANT_ID,
            config.AZURE_CLIENT_ID.clone(),
            Secret::new(&config.AZURE_CLIENT_SECRET),
            None,
        )?;

        Ok(Self {
            credential,
            endpoint: config.STORAGE_ACCOUNT_ENDPOINT.clone(),
            container_name: config.STORAGE_CONTAINER.clone(),
        })
    }

    pub fn blob_container_client(&self) -> Result<BlobContainerClient> {
        Ok(BlobContainerClient::new(
            &self.endpoint,
            &self.container_name,
            Some(self.credential.clone()),
            Some(BlobContainerClientOptions::default()),
        )?)
    }

    pub async fn upload_block(
        &self,
        block_blob_client: &BlockBlobClient,
        buffer: &mut Vec<u8>,
        index: &mut u32,
    ) -> Result<Vec<u8>> {
        let block_id_raw = format!("{index:04}");
        let block_id = encode(&block_id_raw).into_bytes();

        let chunk = Bytes::copy_from_slice(buffer);
        block_blob_client
            .stage_block(&block_id, chunk.len() as u64, chunk.into(), None)
            .await?;

        buffer.clear();
        *index += 1;

        Ok(block_id)
    }

    pub async fn upload_file_by_stream(
        &self,
        container_client: BlobContainerClient,
        filename: impl AsRef<str>,
        mut stream: impl Stream<Item = reqwest::Result<Bytes>> + Unpin,
    ) -> Result<()> {
        let blob_client = container_client.blob_client(filename.as_ref());
        let block_blob_client = blob_client.block_blob_client();
        let worker_name = load_config().WORKER_NAME.as_str();

        let mut block_ids: Vec<Vec<u8>> = vec![];
        let mut chunk_index = 0_u32;
        let mut buffer = Vec::with_capacity(MAX_SIZE_PER_CHUNK);

        while let Some(chunk) = stream.next().await {
            let chunk_res = chunk?;

            #[cfg(feature = "telemetry")]
            CC_BYTES_UPLOADED_COUNTER.add(
                chunk_res.len() as u64,
                &[KeyValue::new("worker_name", worker_name)],
            );

            buffer.extend_from_slice(&chunk_res);

            if buffer.len() >= MAX_SIZE_PER_CHUNK {
                let chunk_id = self
                    .upload_block(&block_blob_client, &mut buffer, &mut chunk_index)
                    .await?;
                block_ids.push(chunk_id);
            }
        }

        if !buffer.is_empty() {
            #[cfg(feature = "telemetry")]
            CC_BYTES_UPLOADED_COUNTER.add(
                buffer.len() as u64,
                &[KeyValue::new("worker_name", worker_name)],
            );

            let chunk_id = self
                .upload_block(&block_blob_client, &mut buffer, &mut chunk_index)
                .await?;
            block_ids.push(chunk_id);
        }

        let block_lookup_list = BlockLookupList {
            latest: Some(block_ids),
            committed: Some(vec![]),
            uncommitted: Some(vec![]),
        };

        let commit_options = BlockBlobClientCommitBlockListOptions {
            blob_content_type: Some("application/gzip".into()),
            ..Default::default()
        };

        block_blob_client
            .commit_block_list(block_lookup_list.try_into()?, Some(commit_options))
            .await?;

        #[cfg(feature = "telemetry")]
        CC_FILES_UPLOADED_COUNTER.add(
            1_u64,
            &[
                KeyValue::new("worker_name", worker_name),
                KeyValue::new("filename", filename.as_ref().to_string()),
            ],
        );

        Ok(())
    }
}
