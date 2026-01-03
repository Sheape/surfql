use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Seed(SeedArgs),
    Download(DownloadArgs),
}

#[derive(Args)]
pub struct SeedArgs {
    #[arg(long = "filename", short = 'f')]
    pub filename: String,
}

#[derive(Args)]
pub struct DownloadArgs {
    #[arg(long = "worker-name", short = 'w')]
    pub worker_name: String,

    #[arg(long, short = 's')]
    pub single: Option<String>,
}
