//! CPU profiler for the multiplexed GET hot path — the exact workload of the
//! `Multiplexing Comparison` benchmark (concurrent tasks, each awaiting
//! sequential GETs), where rustis is measured against fred and redis-rs.
//!
//! Uses `pprof` in userspace (SIGPROF sampling) so it needs neither `perf` nor
//! root. It drives the workload in a loop under a sampling guard, writes a
//! flamegraph SVG, and prints the hottest leaf functions (self time) so the
//! client-side bottleneck is readable without opening the SVG.
//!
//! Run with:
//!   cargo run --release --features bench --example pprof_multiplexer_get
//!
//! Env:
//!   REDIS_HOST   Redis host (default 127.0.0.1)
//!   PPROF_ITERS  outer iterations of the whole fan-out (default 400)
//!   PPROF_TASKS  concurrent tasks (default 12)
//!   PPROF_REQS   sequential GETs per task (default 200)
//!   PPROF_HZ     sampling frequency in Hz (default 4000)

use rustis::{Result, client::Client, commands::StringCommands};
use std::{collections::HashMap, sync::Arc};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

async fn fan_out(client: Client, tasks: usize, reqs: usize, keys: Arc<Vec<String>>) {
    let mut handles = Vec::with_capacity(tasks);
    for _ in 0..tasks {
        let client = client.clone();
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let iters = env_usize("PPROF_ITERS", 400);
    let tasks = env_usize("PPROF_TASKS", 12);
    let reqs = env_usize("PPROF_REQS", 200);
    let hz = env_usize("PPROF_HZ", 4_000) as i32;

    let client = Client::connect(redis_host).await?;
    let keys: Arc<Vec<String>> = Arc::new((0..100).map(|i| format!("key{i}")).collect());
    let data: Vec<_> = (0..100)
        .map(|i| (format!("key{i}"), format!("value{i}")))
        .collect();
    let _: () = client.mset(data).await?;

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(hz)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("failed to start pprof profiler");

    println!("Profiling {iters}×({tasks} tasks × {reqs} GETs) at {hz} Hz...");
    for _ in 0..iters {
        fan_out(client.clone(), tasks, reqs, keys.clone()).await;
    }

    let report = guard
        .report()
        .build()
        .expect("failed to build pprof report");

    let out_path = "target/pprof_multiplexer_get_flamegraph.svg";
    let file = std::fs::File::create(out_path).expect("failed to create flamegraph file");
    report
        .flamegraph(file)
        .expect("failed to write flamegraph SVG");
    println!("Flamegraph written to {out_path}\n");

    let mut leaf: HashMap<String, isize> = HashMap::new();
    let mut total = 0isize;
    for (frames, count) in report.data.iter() {
        total += *count;
        if let Some(sym) = frames.frames.first().and_then(|f| f.first()) {
            *leaf.entry(format!("{sym}")).or_default() += *count;
        }
    }
    let mut rows: Vec<(String, isize)> = leaf.into_iter().collect();
    rows.sort_by_key(|(_, c)| std::cmp::Reverse(*c));

    println!("=== hottest leaf functions — self time ({total} samples) ===");
    for (sym, count) in rows.iter().take(35) {
        let pct = 100.0 * *count as f64 / total.max(1) as f64;
        println!("{pct:6.2}%  {count:>7}  {sym}");
    }

    // Trace the callers of any leaf mentioning `powf` — an anomaly in a GET path.
    println!("\n=== callers of `powf`-family leaves ===");
    let mut callers: HashMap<String, isize> = HashMap::new();
    for (frames, count) in report.data.iter() {
        let stack = &frames.frames;
        if let Some(leaf) = stack.first().and_then(|f| f.first())
            && format!("{leaf}").contains("powf")
        {
            // Print the first few frames above the leaf.
            let chain: Vec<String> = stack
                .iter()
                .take(6)
                .filter_map(|f| f.first().map(|s| format!("{s}")))
                .collect();
            *callers.entry(chain.join("  <-  ")).or_default() += *count;
        }
    }
    for (chain, count) in callers.iter() {
        println!("{count:>5}  {chain}");
    }

    Ok(())
}
