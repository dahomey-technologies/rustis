#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code: a panic is how a test reports failure"
)]

// The suite is split so that `server-tests` alone selects it. A module gated on
// that feature needs a live Redis; everything left when it is off needs neither a
// server nor a network, and passes. A `*_server` module is the server-bound half
// of the module it is named after, whose own half stays hermetic.
mod arg_serializer;
mod array_commands;
#[cfg(feature = "server-tests")]
mod array_commands_server;
#[cfg(feature = "tokio-runtime")]
mod backpressure;
#[cfg(all(feature = "tokio-runtime", feature = "server-tests"))]
mod backpressure_server;
#[cfg(feature = "server-tests")]
mod bitmap_commands;
#[cfg(feature = "server-tests")]
mod bloom_commands;
mod buffer_decoder;
#[cfg(all(feature = "client-cache", feature = "server-tests"))]
mod cache;
#[cfg(feature = "server-tests")]
mod client;
#[cfg(feature = "tokio-runtime")]
mod close;
mod cluster;
mod cluster_commands;
#[cfg(feature = "server-tests")]
mod cluster_commands_server;
#[cfg(feature = "server-tests")]
mod cluster_server;
#[cfg(feature = "server-tests")]
mod command_args;
mod command_future;
#[cfg(feature = "server-tests")]
mod command_future_server;
mod config;
#[cfg(feature = "server-tests")]
mod config_server;
#[cfg(feature = "server-tests")]
mod connection_commands;
#[cfg(feature = "server-tests")]
mod count_min_sketch_commands;
#[cfg(feature = "server-tests")]
mod cuckoo_commands;
#[cfg(feature = "server-tests")]
mod debug_commands;
mod error;
#[cfg(feature = "server-tests")]
mod error_server;
#[cfg(feature = "server-tests")]
mod exclusive_client;
#[cfg(feature = "tokio-runtime")]
mod fake_server;
#[cfg(feature = "tokio-runtime")]
mod fault_injection_proxy;
#[cfg(all(feature = "tokio-runtime", feature = "server-tests"))]
mod fault_injection_proxy_server;
mod from_value;
mod generic_commands;
#[cfg(feature = "server-tests")]
mod generic_commands_server;
#[cfg(feature = "server-tests")]
mod geo_commands;
#[cfg(feature = "server-tests")]
mod hash_commands;
#[cfg(feature = "server-tests")]
mod hyper_log_log_commands;
mod instrumentation;
#[cfg(all(feature = "json", feature = "server-tests"))]
mod json;
mod json_commands;
#[cfg(feature = "server-tests")]
mod json_commands_server;
#[cfg(feature = "server-tests")]
mod list_commands;
#[cfg(feature = "server-tests")]
mod multiplexed_client;
#[cfg(feature = "server-tests")]
mod network_handler;
#[cfg(feature = "server-tests")]
mod pipeline;
#[cfg(feature = "pool")]
mod pooled_client_manager;
#[cfg(all(feature = "pool", feature = "server-tests"))]
mod pooled_client_manager_server;
#[cfg(feature = "server-tests")]
mod pub_sub_commands;
mod pub_sub_message;
mod reconnection_state;
#[cfg(feature = "server-tests")]
mod resp3;
mod resp_deserializer;
mod resp_frame_parser;
mod resp_response;
mod resp_tape;
pub(crate) mod response_probe;
mod response_shape;
#[cfg(feature = "server-tests")]
mod scripting_commands;
mod search_commands;
#[cfg(feature = "server-tests")]
mod search_commands_server;
mod sentinel;
#[cfg(feature = "server-tests")]
mod sentinel_server;
mod server_commands;
#[cfg(feature = "server-tests")]
mod server_commands_server;
#[cfg(feature = "server-tests")]
mod set_commands;
mod socket_options;
mod sorted_set_commands;
#[cfg(feature = "server-tests")]
mod sorted_set_commands_server;
mod store_destination_keys;
mod stream_commands;
#[cfg(feature = "server-tests")]
mod stream_commands_server;
mod string_commands;
#[cfg(feature = "server-tests")]
mod string_commands_server;
#[cfg(feature = "server-tests")]
mod t_digest_commands;
#[cfg(feature = "server-tests")]
mod time_series_commands;
#[cfg(feature = "server-tests")]
mod tls;
#[cfg(feature = "server-tests")]
mod top_k_commands;
#[cfg(feature = "server-tests")]
mod transaction;
#[cfg(feature = "tokio-runtime")]
mod transport;
mod util;
#[cfg(feature = "server-tests")]
mod util_server;
mod value;
mod value_deserialize;
mod value_deserializer;
#[cfg(feature = "server-tests")]
mod value_server;
mod vector_sets;
#[cfg(feature = "server-tests")]
mod vector_sets_server;

pub(crate) use util::*;
#[cfg(feature = "server-tests")]
pub(crate) use util_server::*;
