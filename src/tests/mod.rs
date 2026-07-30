#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic is how a test reports failure"
)]

mod arg_serializer;
mod array_commands;
#[cfg(feature = "tokio-runtime")]
mod backpressure;
mod bitmap_commands;
mod bloom_commands;
mod buffer_decoder;
#[cfg(feature = "client-cache")]
mod cache;
mod client;
mod cluster;
mod cluster_commands;
mod command_args;
mod config;
mod connection_commands;
mod count_min_sktech_commands;
mod cuckoo_commands;
mod debug_commands;
mod error;
#[cfg(feature = "tokio-runtime")]
mod fault_injection_proxy;
mod from_value;
mod generic_commands;
mod geo_commands;
mod hash_commands;
mod hyper_log_log_commands;
mod instrumentation;
#[cfg(feature = "json")]
mod json;
mod json_commands;
mod list_commands;
mod multiplexed_client;
mod network_handler;
mod pipeline;
#[cfg(feature = "pool")]
mod pooled_client_manager;
mod pub_sub_commands;
mod resp3;
mod resp_deserializer;
mod resp_frame_parser;
mod resp_response;
mod resp_tape;
pub(crate) mod response_probe;
mod response_shape;
mod scripting_commands;
mod search_commands;
mod sentinel;
mod server_commands;
mod set_commands;
mod sorted_set_commands;
mod stream_commands;
mod string_commands;
mod t_disgest_commands;
mod time_series_commands;
mod tls;
mod top_k_commands;
mod transaction;
mod util;
mod value;
mod value_deserialize;
mod value_deserializer;
mod vector_sets;

pub(crate) use util::*;
