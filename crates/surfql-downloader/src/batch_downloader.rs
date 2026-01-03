use std::sync::Arc;

use surfql_queue::{Channel, ChannelConsumeExt, MessageQueue};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use crate::{Result, consumer::WarcConsumer, downloader::Downloader, signal::shutdown_signal};

pub struct BatchDownloader {
    queue_name: String,
    queue_limit: u16,
    concurrency_limit: u32,
}

impl BatchDownloader {
    pub fn new(queue_limit: u16, queue_name: String, concurrency_limit: u32) -> Self {
        Self {
            queue_name,
            queue_limit,
            concurrency_limit,
        }
    }

    async fn run_batch(
        self,
        channel: Channel,
        downloader: Downloader,
        semaphore: Arc<Semaphore>,
        token: CancellationToken,
    ) {
        let consumer = WarcConsumer::new(downloader, semaphore.clone());
        // TODO: Handle error case
        channel
            .warc_consume(consumer, self.queue_limit, &self.queue_name)
            .await
            .unwrap();

        token.cancelled().await;

        // TODO: Handle error case
        channel.cancel_worker().await.unwrap();
        let _ = semaphore.acquire_many(self.concurrency_limit).await;
    }

    pub async fn run(self) -> Result<()> {
        let queue_name = self.queue_name.clone();
        let message_queue = MessageQueue::new(queue_name).await?;
        let downloader = Downloader::new().await?;
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit as usize));
        let token = CancellationToken::new();

        let mut worker_task = tokio::spawn(self.run_batch(
            message_queue.channel.clone(),
            downloader,
            semaphore.clone(),
            token.clone(),
        ));

        tokio::select! {
            _ = shutdown_signal() => {
                println!("Cleaning up before exit...");
                token.cancel();
            }
            res = &mut worker_task => {
                match res {
                    Ok(_) => println!("Worker finished successfully"),
                    Err(e) => eprintln!("Worker task panicked: {e:?}")
                }
            }
        }

        message_queue.close_connection().await?;

        Ok(())
    }
}
