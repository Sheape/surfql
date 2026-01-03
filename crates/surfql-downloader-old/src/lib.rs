mod cli;
mod config;
mod consumer;
mod downloader;
mod error;
mod seeder;

use std::sync::Arc;

use amqprs::channel::{BasicCancelArguments, BasicConsumeArguments, BasicQosArguments};
use clap::Parser;
pub use error::{Error, Result};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{signal, sync::Semaphore};
use tokio_util::sync::CancellationToken;

use crate::{
    cli::{Cli, Commands},
    config::load_config,
    consumer::WarcConsumer,
    downloader::Downloader,
    seeder::Seeder,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Seed(_) => {
            let seeder = Seeder::new().await?;
            seeder.publish_paths().await?;
            seeder.close_rabbitmq_channel_connection().await?;
        }
        Commands::Download(args) => {
            if let Some(path) = &args.single {
                let downloader = Downloader::new().await?;
                let pb = ProgressBar::new(0);
                pb.set_style(ProgressStyle::with_template(
                "{msg}: {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta_precise})"
            ).unwrap().progress_chars("██░"));
                pb.set_message(format!("File ...{}", &path[path.len() - 20..]));

                downloader.download_file(path, pb.clone()).await?;
                pb.finish_with_message(format!("{:<29}", "Done!"));
            } else {
                let config = load_config();
                let seeder = Seeder::new().await?;
                let connection = seeder.connection;
                let channel = seeder.channel;
                let worker_name = args.node_name.clone();

                let token = CancellationToken::new();
                let worker_token = token.clone();

                let downloader = Downloader::new().await.unwrap();
                let semaphore = Arc::new(Semaphore::new(10));
                let semaphore_worker = semaphore.clone();
                let channel_worker = channel.clone();
                let mut worker_handle = tokio::spawn(async move {
                    channel_worker
                        .basic_qos(BasicQosArguments::new(0, 12, false))
                        .await
                        .unwrap();

                    let consumer = WarcConsumer::new(downloader, semaphore);

                    let args = BasicConsumeArguments::new(&config.QUEUE_NAME, &worker_name)
                        .manual_ack(true)
                        .finish();

                    channel_worker.basic_consume(consumer, args).await.unwrap();

                    worker_token.cancelled().await;

                    channel_worker
                        .basic_cancel(BasicCancelArguments::new(&worker_name))
                        .await
                        .unwrap();

                    for _ in 0..10 {
                        let _ = semaphore_worker.acquire().await.unwrap();
                    }
                });

                tokio::select! {
                    _ = signal::ctrl_c() => {
                        println!("\n[Ctrl+C] Shutdown signal received. Cleaning up...");
                        token.cancel();
                    }
                    _ = &mut worker_handle => {
                        println!("Worker finished successfully");
                    }
                }

                channel.close().await?;
                connection.close().await?;
            }
        }
    }

    Ok(())
}
