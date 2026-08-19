// A panic here kills the network task, and with it every in-flight command and
// the reconnection loop itself — there is nothing left to retry on. Indexing is
// therefore denied, not warned; see the panic policy in `lib.rs`.
#![deny(clippy::indexing_slicing)]
// `as` is denied here for the same reason it is in `resp/`: a narrowing,
// sign-changing or float-to-integer cast is silent where it is wrong, and this
// task handles values the server chose. Every conversion is a `From`/`TryFrom`,
// or an `as` whose exactness an `#[expect(…, reason = "…")]` states.
#![deny(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless
)]

mod async_executor_strategy;
mod cluster_connection;
mod connection;
mod connection_mode;
mod connection_state;
mod message_queue;
mod network_handler;
mod pub_sub_push;
mod reconnection_state;
mod reply_mode;
mod retry_policy;
mod router;
mod sentinel_connection;
mod standalone_connection;
mod version;

pub(crate) use async_executor_strategy::*;
pub(crate) use cluster_connection::*;
pub(crate) use connection::*;
pub(crate) use connection_state::*;
pub(crate) use network_handler::*;
pub(crate) use pub_sub_push::*;
pub(crate) use reconnection_state::*;
pub(crate) use sentinel_connection::*;
pub(crate) use standalone_connection::*;
pub(crate) use version::*;
