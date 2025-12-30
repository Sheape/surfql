use std::time::Duration;

use amqprs::{
    BasicProperties, DELIVERY_MODE_PERSISTENT,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, Channel, ConfirmSelectArguments, QueueDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::time::sleep;

use crate::{Result, config::load_config};

pub struct Seeder {
    pub connection: Connection,
    pub channel: Channel,
}

impl Seeder {
    pub async fn new() -> Result<Self> {
        let config = load_config();
        let args: OpenConnectionArguments =
            config.RABBITMQ_CONNECTION_STRING.as_str().try_into()?;
        let connection = Connection::open(&args).await?;
        connection
            .register_callback(DefaultConnectionCallback)
            .await?;

        let channel = connection.open_channel(None).await?;
        channel.register_callback(DefaultChannelCallback).await?;

        let queue_arg = QueueDeclareArguments::new(&config.QUEUE_NAME)
            .durable(true)
            .finish();
        channel.queue_declare(queue_arg).await?;
        channel
            .confirm_select(ConfirmSelectArguments::new(true))
            .await?;

        Ok(Self {
            channel,
            connection,
        })
    }

    pub async fn publish_paths(&self) -> Result<()> {
        let warc_paths = include_str!("../../../samples/warc.paths");
        let pb = ProgressBar::new(warc_paths.lines().count() as u64);
        let config = load_config();
        pb.set_style(ProgressStyle::with_template(
                "{msg}: {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} ({eta})"
            ).unwrap().progress_chars("██░"));

        for path in warc_paths.lines() {
            pb.set_message(format!("Seeding: ...{}", &path[path.len() - 20..]));
            let props = BasicProperties::default()
                .with_delivery_mode(DELIVERY_MODE_PERSISTENT)
                .with_content_type("text/plain")
                .finish();
            let content = Vec::from(path.as_bytes());
            let publish_args = BasicPublishArguments::new("", &config.QUEUE_NAME);

            self.channel
                .basic_publish(props, content, publish_args)
                .await?;

            pb.inc(1);
        }
        pb.finish_with_message("Done!");

        Ok(())
    }

    pub async fn close_rabbitmq_channel_connection(self) -> Result<()> {
        sleep(Duration::from_secs(2)).await;
        self.channel.close().await?;
        Ok(self.connection.close().await?)
    }
}
