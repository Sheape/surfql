use std::{fs, path::Path};

use surfql_queue::MessageQueue;

use crate::{Error, Result};

pub struct Seeder {
    message_queue: MessageQueue,
}

impl Seeder {
    pub async fn new(queue_name: impl Into<String>) -> Result<Self> {
        let message_queue = MessageQueue::new(queue_name).await?;
        Ok(Self { message_queue })
    }

    pub async fn seed_from_file(&self, filepath: String) -> Result<()> {
        let path = Path::new(&filepath);
        let contents = fs::read_to_string(path).map_err(|err| Error::InvalidFileInput {
            filepath,
            source: err,
        })?;

        for path in contents.lines() {
            // PERF: Optimize this to use static bytes only and not Vec<u8>
            self.message_queue
                .publish_persistent(None, path.as_bytes().to_vec())
                .await?;
        }

        Ok(())
    }

    pub async fn done(self) -> Result<()> {
        Ok(self.message_queue.close_connection().await?)
    }
}
