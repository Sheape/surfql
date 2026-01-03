mod channel;
mod error;
mod message_queue;

pub use channel::ChannelConsumeExt;
pub use error::{Error, Result};
pub use message_queue::MessageQueue;

pub use amqprs::{BasicProperties, Deliver, channel::Channel, consumer::AsyncConsumer};
