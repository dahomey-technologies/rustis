//! A Redis client for Rust

mod clients;
mod commands;
mod error;
mod network;
pub mod resp;

pub use clients::*;
pub use commands::*;
pub use error::*;
use network::*;
#[cfg(feature = "pool")]
pub use bb8;

/// Library general result type.
pub type Result<T> = std::result::Result<T, Error>;
/// Library general future type.
pub type Future<'a, T> = futures::future::BoxFuture<'a, Result<T>>;

#[cfg(all(feature = "tokio-runtime", feature = "async-std-runtime"))]
compile_error!("feature \"tokio-runtime\" and feature \"async-std-runtime\" cannot be enabled at the same time");

#[cfg(test)]
mod tests;

