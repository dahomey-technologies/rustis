//! A Redis client for Rust inspired by [StackExchange.Redis](https://github.com/StackExchange/StackExchange.Redis)
//! & [redis-async-rs](https://github.com/benashford/redis-async-rs)

mod commands;
mod connection_multiplexer;
mod database;
mod error;
mod message;
mod network;
mod pub_sub;
pub mod resp;
mod transaction;

#[cfg(test)]
mod tests;

pub use commands::*;
pub use connection_multiplexer::*;
pub use database::*;
pub use error::*;
pub(crate) use message::*;
pub use network::*;
pub use pub_sub::*;
pub use transaction::*;

use futures::channel::{mpsc, oneshot};

type MsgSender = mpsc::UnboundedSender<Message>;
type MsgReceiver = mpsc::UnboundedReceiver<Message>;
type ValueSender = oneshot::Sender<Result<resp::Value>>;
type ValueReceiver = oneshot::Receiver<Result<resp::Value>>;
type PubSubSender = mpsc::UnboundedSender<Result<resp::BulkString>>;
type PubSubReceiver = mpsc::UnboundedReceiver<Result<resp::BulkString>>;
