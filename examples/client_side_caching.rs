//! Caching reads in the process, invalidated by the server.
//!
//! The client subscribes to `CLIENT TRACKING`: the server names every key this
//! connection has read as soon as anyone writes it, and the cache drops it. So a
//! hit costs no round trip and a stale value is bounded by the invalidation
//! message, not by a TTL guess.
//!
//! Invalidations are per-connection state. Losing them — a reconnection, or a
//! burst shed under backpressure — means no longer knowing what is stale, and
//! the cache empties itself rather than serve a value it cannot vouch for.
//!
//! ```sh
//! cargo run --example client_side_caching --features client-cache
//! ```
use rustis::{
    Result,
    cache::Cache,
    client::Client,
    commands::{ClientTrackingOptions, StringCommands},
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let reader = Client::connect("127.0.0.1:6379").await?;
    let writer = Client::connect("127.0.0.1:6379").await?;

    writer.set("cached_key", "first").await?;

    // 60 s TTL, on top of the invalidations: the TTL bounds an entry nobody ever
    // writes again, which no invalidation would ever name.
    let cache = Cache::new(reader, 60, ClientTrackingOptions::default()).await?;

    // Miss: read from the server, then kept.
    let value: String = cache.get("cached_key").await?;
    println!("{value}");

    // Hit: no round trip at all.
    let value: String = cache.get("cached_key").await?;
    println!("{value}");

    // Another client writes. The server tells this connection the key is stale.
    writer.set("cached_key", "second").await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Miss again, and the new value.
    let value: String = cache.get("cached_key").await?;
    println!("{value}");

    Ok(())
}
