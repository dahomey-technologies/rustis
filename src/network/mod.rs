// A panic here kills the network task, and with it every in-flight command and
// the reconnection loop itself — there is nothing left to retry on. Indexing is
// therefore denied, not warned; see the panic policy in `lib.rs`.
#![deny(clippy::indexing_slicing)]

mod async_executor_strategy;
mod cluster_connection;
mod connection;
mod connection_state;
mod network_handler;
mod pub_sub_message;
mod reconnection_state;
mod sentinel_connection;
mod standalone_connection;
mod version;

pub(crate) use async_executor_strategy::*;
pub(crate) use cluster_connection::*;
pub(crate) use connection::*;
pub(crate) use connection_state::*;
pub(crate) use network_handler::*;
pub(crate) use reconnection_state::*;
pub(crate) use sentinel_connection::*;
pub(crate) use standalone_connection::*;
pub(crate) use version::*;
