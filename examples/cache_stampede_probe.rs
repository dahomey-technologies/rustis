//! Cache-stampede measurement.
//!
//! The client cache checks moka and, on a miss, sends the `GET` to the server
//! *before* the `entry()` insert. Concurrent misses on the same key therefore
//! each reach the server: there is no single-flight coalescing. Whether that
//! matters is a workload question, so this probe measures it instead of guessing.
//!
//! For a range of concurrency levels it fires N simultaneous `get`s on one
//! never-cached key against a fresh cache and counts, via server-side
//! `INFO commandstats`, how many `GET`s actually hit the server — the stampede
//! amplification factor. It also times the cold burst against an all-hit warm
//! burst of the same width, which bounds the latency a single-flight coalescer
//! could remove (the warm burst does zero server round-trips).
//!
//! Run: `cargo run --release --example cache_stampede_probe --features client-cache`
//! Needs a Redis server on 127.0.0.1:6379.

use rustis::{
    Result,
    cache::Cache,
    client::Client,
    commands::{
        ClientTrackingOptions, ClientTrackingStatus, ConnectionCommands, FlushingMode,
        ServerCommands, StringCommands,
    },
    resp::cmd,
};
use std::time::Instant;

/// Number of `GET` calls the server has processed since the last `CONFIG RESETSTAT`,
/// read from `INFO commandstats` (`cmdstat_get:calls=N,...`). Returns 0 when the
/// command has not been called yet (the line is absent right after a reset).
async fn server_get_calls(control: &Client) -> Result<u64> {
    let info: String = control.send(cmd("INFO").arg("commandstats"), None).await?;
    for line in info.lines() {
        if let Some(rest) = line.strip_prefix("cmdstat_get:")
            && let Some(calls) = rest.split(',').find_map(|kv| kv.strip_prefix("calls="))
        {
            return Ok(calls.trim().parse().unwrap_or(0));
        }
    }
    Ok(0)
}

#[tokio::main]
async fn main() -> Result<()> {
    // `control` drives the server (set/flush/stats); `cache_client` backs the cache.
    let control = Client::connect("redis://127.0.0.1?connection_name=stampede_control").await?;
    let cache_client = Client::connect("redis://127.0.0.1?connection_name=stampede_cache").await?;

    control
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    println!(
        "{:>5} | {:>12} | {:>15} | {:>13} | {:>12}",
        "N", "server GETs", "stampede factor", "cold burst", "warm burst"
    );
    println!("{}", "-".repeat(70));

    for &concurrency in &[1usize, 2, 4, 8, 16, 32, 64, 128] {
        control.flushall(FlushingMode::Sync).await?;
        let _: () = control.set("hot", "value").await?;

        // A fresh cache per round guarantees the key is a cold miss for everyone.
        let cache = Cache::new(cache_client.clone(), 60, ClientTrackingOptions::default()).await?;

        control
            .send::<()>(cmd("CONFIG").arg("RESETSTAT"), None)
            .await?;
        let before = server_get_calls(&control).await?;

        // Cold burst: N concurrent misses on the same key.
        let start = Instant::now();
        let mut handles = Vec::with_capacity(concurrency);
        for _ in 0..concurrency {
            let cache = cache.clone();
            handles.push(tokio::spawn(
                async move { cache.get::<String>("hot").await },
            ));
        }
        for handle in handles {
            let value = handle.await.expect("task panicked")?;
            assert_eq!(value, "value");
        }
        let cold = start.elapsed();

        let after = server_get_calls(&control).await?;
        let server_gets = after.saturating_sub(before);

        // Warm burst: same width, but every key is now cached — zero server GETs.
        let start = Instant::now();
        let mut handles = Vec::with_capacity(concurrency);
        for _ in 0..concurrency {
            let cache = cache.clone();
            handles.push(tokio::spawn(
                async move { cache.get::<String>("hot").await },
            ));
        }
        for handle in handles {
            handle.await.expect("task panicked")?;
        }
        let warm = start.elapsed();

        println!(
            "{concurrency:>5} | {server_gets:>12} | {:>14.1}x | {:>10.2?} | {:>10.2?}",
            server_gets as f64 / concurrency as f64,
            cold,
            warm,
        );
    }

    println!(
        "\nReading: a stampede factor near 1.0x means the burst is already coalesced;\n\
         near N (server GETs ~= N) means every concurrent miss hits the server. The\n\
         cold-vs-warm gap is the round-trip latency a single-flight coalescer could\n\
         remove for the redundant callers."
    );

    Ok(())
}
