mod cli;

use std::{env, error::Error};

use clap::Parser;
use surfql_downloader::{BatchDownloader, Downloader, Seeder};
use surfql_telemetry::init_telemetry;

use crate::cli::{Cli, Commands};

const QUEUE_NAME: &str = "warc_tasks";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_telemetry();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Seed(args) => {
            let seeder = Seeder::new(QUEUE_NAME).await?;
            let filename = args.filename.clone();
            seeder.seed_from_file(filename).await?;
            seeder.done().await?;
        }

        Commands::Download(args) => {
            let worker_name = args.worker_name.clone();
            unsafe {
                env::set_var("WORKER_NAME", worker_name);
            }

            if let Some(url) = &args.single {
                let downloader = Downloader::new().await?;
                let filename = url.rsplit_once('/').unwrap().0;
                downloader.download_file_stream(url, filename).await?;
            } else {
                let batch_downloader = BatchDownloader::new(12, QUEUE_NAME.to_string(), 10);
                batch_downloader.run().await?;
            }
        }
    }

    Ok(())
}
