use std::time::Duration;

use surfql_core::load_config;

use crate::Result;

use amqprs::{
    BasicProperties, DELIVERY_MODE_PERSISTENT,
    callbacks::{DefaultChannelCallback, DefaultConnectionCallback},
    channel::{BasicPublishArguments, Channel, ConfirmSelectArguments, QueueDeclareArguments},
    connection::{Connection, OpenConnectionArguments},
};
use tokio::time::sleep;

pub struct MessageQueue {
    pub connection: Connection,
    pub channel: Channel,
    queue_name: String,
}

impl MessageQueue {
    pub async fn new(queue_name: impl Into<String>) -> Result<Self> {
        let connection_str = load_config().RABBITMQ_CONNECTION_STRING.as_str();
        let args: OpenConnectionArguments = connection_str.try_into()?;
        let connection = Connection::open(&args).await?;
        connection
            .register_callback(DefaultConnectionCallback)
            .await?;

        let channel = connection.open_channel(None).await?;
        channel.register_callback(DefaultChannelCallback).await?;

        let queue_name = queue_name.into();
        let queue_arg = QueueDeclareArguments::new(queue_name.as_str())
            .durable(true)
            .finish();
        channel.queue_declare(queue_arg).await?;
        channel
            .confirm_select(ConfirmSelectArguments::new(true))
            .await?;

        Ok(Self {
            channel,
            connection,
            queue_name,
        })
    }

    pub async fn publish_persistent(&self, exchange: Option<&str>, content: Vec<u8>) -> Result<()> {
        let props = BasicProperties::default()
            .with_delivery_mode(DELIVERY_MODE_PERSISTENT)
            .with_content_type("text/plain")
            .finish();
        let publish_args = BasicPublishArguments::new(exchange.unwrap_or(""), &self.queue_name);

        self.channel
            .basic_publish(props, content, publish_args)
            .await?;

        Ok(())
    }

    pub async fn close_connection(self) -> Result<()> {
        sleep(Duration::from_secs(2)).await;
        self.channel.close().await?;
        Ok(self.connection.close().await?)
    }
}
