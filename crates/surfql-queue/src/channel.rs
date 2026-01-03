use std::process::Output;

use amqprs::{
    Deliver,
    channel::{
        BasicAckArguments, BasicCancelArguments, BasicConsumeArguments, BasicNackArguments,
        BasicQosArguments, Channel,
    },
    consumer::AsyncConsumer,
};
use surfql_core::load_config;
type Result<T> = core::result::Result<T, amqprs::error::Error>;

pub trait ChannelConsumeExt {
    fn ack_delivery(&self, delivery: Deliver) -> impl Future<Output = Result<()>> + Send;
    fn nack_delivery(&self, delivery: Deliver) -> impl Future<Output = Result<()>> + Send;
    fn warc_consume<F>(
        &self,
        consumer: F,
        prefetch_count: u16,
        queue_name: &str,
    ) -> impl Future<Output = Result<()>> + Send
    where
        F: AsyncConsumer + Send + 'static;

    fn cancel_worker(&self) -> impl Future<Output = Result<()>> + Send;
}

impl ChannelConsumeExt for Channel {
    async fn ack_delivery(&self, delivery: Deliver) -> Result<()> {
        let args = BasicAckArguments::new(delivery.delivery_tag(), false);
        self.basic_ack(args).await
    }

    async fn nack_delivery(&self, delivery: Deliver) -> Result<()> {
        let args = BasicNackArguments::new(delivery.delivery_tag(), false, true);
        self.basic_nack(args).await
    }

    async fn warc_consume<F>(
        &self,
        consumer: F,
        prefetch_count: u16,
        queue_name: &str,
    ) -> Result<()>
    where
        F: AsyncConsumer + Send + 'static,
    {
        let worker_name = &load_config().WORKER_NAME;
        self.basic_qos(BasicQosArguments::new(0, prefetch_count, false))
            .await?;

        let consumer_args = BasicConsumeArguments::new(queue_name, worker_name)
            .manual_ack(true)
            .finish();

        self.basic_consume(consumer, consumer_args).await?;

        Ok(())
    }

    async fn cancel_worker(&self) -> Result<()> {
        let worker_name = &load_config().WORKER_NAME;
        self.basic_cancel(BasicCancelArguments::new(worker_name))
            .await?;

        Ok(())
    }
}
