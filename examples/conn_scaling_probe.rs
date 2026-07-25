//! Decides whether the single serialized network task is the throughput ceiling.
//!
//! Same total concurrency, but the fan-out tasks are spread over N independent
//! clients (N independent network tasks). If throughput climbs with N, the lone
//! network task was the serial cap and its per-request CPU is the lever. If it
//! stays flat, the ceiling is elsewhere (global CPU / runtime coordination) and
//! trimming the network task cannot help.
//!
//! Run with:
//!   cargo run --release --features bench --example conn_scaling_probe
//!
//! Env:
//!   REDIS_HOST  Redis host (default 127.0.0.1)
//!   PROBE_TASKS total concurrent fan-out tasks (default 256)
//!   PROBE_REQS  sequential GETs per task (default 2000)

use rustis::{Result, client::Client, commands::StringCommands};
use std::{sync::Arc, time::Instant};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn run(clients: &[Client], tasks: usize, reqs: usize, keys: Arc<Vec<String>>) -> f64 {
    let start = Instant::now();
    let mut handles = Vec::with_capacity(tasks);
    for t in 0..tasks {
        // Round-robin the tasks across the available clients.
        let client = clients[t % clients.len()].clone();
        let keys = keys.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..reqs {
                let _: String = client.get(&keys[i % keys.len()]).await.unwrap();
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
    let elapsed = start.elapsed().as_secs_f64();
    (tasks * reqs) as f64 / elapsed
}

#[tokio::main]
async fn main() -> Result<()> {
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let tasks = env_usize("PROBE_TASKS", 256);
    let reqs = env_usize("PROBE_REQS", 2000);

    let keys: Arc<Vec<String>> = Arc::new((0..100).map(|i| format!("key{i}")).collect());

    // Seed the data once.
    let seed = Client::connect(redis_host.clone()).await?;
    let data: Vec<_> = (0..100)
        .map(|i| (format!("key{i}"), format!("value{i}")))
        .collect();
    let _: () = seed.mset(data).await?;

    println!("{tasks} tasks × {reqs} GETs, round-robin over N connections\n");
    for &n in &[1usize, 2, 4, 8] {
        let mut clients = Vec::with_capacity(n);
        for _ in 0..n {
            clients.push(Client::connect(redis_host.clone()).await?);
        }
        // Warm up, then measure the best of 3 to shed scheduler noise.
        let _ = run(&clients, tasks, reqs / 4, keys.clone()).await;
        let mut best = 0.0f64;
        for _ in 0..3 {
            let ops = run(&clients, tasks, reqs, keys.clone()).await;
            if ops > best {
                best = ops;
            }
        }
        println!("{n:>2} conn  {:>10.0} ops/s", best);
    }

    Ok(())
}
