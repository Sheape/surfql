use std::{pin::Pin, sync::Arc, time::Duration};

use amqprs::{
    BasicProperties, Deliver,
    channel::{BasicAckArguments, BasicNackArguments, Channel},
    consumer::AsyncConsumer,
};
use async_trait::async_trait;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::Rng;
use tokio::{sync::Semaphore, time::sleep};

use crate::downloader::Downloader;

pub struct WarcConsumer {
    pub downloader: Arc<Downloader>,
    pub semaphore: Arc<Semaphore>,
    multi_progress: Arc<MultiProgress>,
}

#[async_trait]
impl AsyncConsumer for WarcConsumer {
    #[allow(
        mismatched_lifetime_syntaxes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    fn consume<'life0, 'life1, 'async_trait>(
        &'life0 mut self,
        channel: &'life1 Channel,
        deliver: Deliver,
        _basic_properties: BasicProperties,
        content: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let downloader = self.downloader.clone();
        let semaphore = self.semaphore.clone();
        let channel = channel.clone();
        let mp = self.multi_progress.clone();

        Box::pin(async move {
            tokio::spawn(async move {
                let url = String::from_utf8(content).unwrap();
                let _permit = semaphore.acquire().await.unwrap();

                let jitter = rand::rng().random_range(0..=2000);
                sleep(Duration::from_millis(jitter)).await;

                let pb = mp.add(ProgressBar::new(0));
                pb.set_style(ProgressStyle::with_template(
                "{msg}: {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta_precise})"
            ).unwrap().progress_chars("██░"));
                pb.set_message(format!("File ...{}", &url[url.len() - 20..]));

                let response = downloader.download_file(url, pb.clone()).await;
                match response {
                    Ok(_) => {
                        let args = BasicAckArguments::new(deliver.delivery_tag(), false);
                        channel.basic_ack(args).await.unwrap();
                        pb.finish_with_message(format!("{:<28}", "Done!"));

                        mp.remove(&pb);
                    }
                    Err(e) => {
                        let args = BasicNackArguments::new(deliver.delivery_tag(), false, true);
                        channel.basic_nack(args).await.unwrap();
                        pb.abandon_with_message(format!("Error: {e}"));
                    }
                }
            });
        })
    }
}

impl WarcConsumer {
    pub fn new(downloader: Downloader, semaphore: Arc<Semaphore>) -> Self {
        Self {
            downloader: Arc::new(downloader),
            semaphore,
            multi_progress: Arc::new(MultiProgress::new()),
        }
    }
}
