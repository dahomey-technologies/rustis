//! Attributes the serialized network-task CPU to its two halves — **send** vs
//! **receive** — under a *saturated* workload (many concurrent tasks), the
//! regime where rustis trails redis-rs on raw throughput.
//!
//! Prior latency-bound profiling (12 tasks) found the serial network-task path
//! was send-dominated (write syscall ~70%). Under saturation the write coalesces
//! many more commands per flush, so the per-request picture shifts; this example
//! re-measures it and buckets every sample so the send/receive split is explicit.
//!
//! Uses `pprof` in userspace (SIGPROF sampling): no `perf`, no root. Each sample
//! is a full stack; we classify it by scanning for unambiguous rustis anchors on
//! the send path (`send_messages`/`handle_message`/`feed`/`flush`) vs the receive
//! path (`handle_result`/`receive_result`/`try_read`/`dispatch_pending`). Samples
//! with neither are split into caller-side work vs runtime/scheduler.
//!
//! Run with:
//!   cargo run --release --features bench --example pprof_send_vs_receive
//!
//! Env:
//!   REDIS_HOST   Redis host (default 127.0.0.1)
//!   PPROF_ITERS  outer iterations of the whole fan-out (default 200)
//!   PPROF_TASKS  concurrent tasks (default 256)
//!   PPROF_REQS   sequential GETs per task (default 100)
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Bucket {
    Send,
    Receive,
    Caller,
    Runtime,
}

impl Bucket {
    fn label(self) -> &'static str {
        match self {
            Bucket::Send => "SEND     (network write half)",
            Bucket::Receive => "RECEIVE  (network read half)",
            Bucket::Caller => "CALLER   (command construction / await)",
            Bucket::Runtime => "RUNTIME  (scheduler / channels / syscalls)",
        }
    }
}

/// Classify a full stack (leaf-first list of symbol strings, lowercased).
fn classify(frames: &[String]) -> Bucket {
    // Unambiguous rustis anchors: presence anywhere in the stack decides the
    // network-task half. The `select!` runs exactly one branch per poll, so a
    // stack cannot legitimately contain both a send and a receive anchor.
    const SEND: &[&str] = &[
        "send_messages",
        "handle_message",
        "connection::feed",
        "standalone_connection",
        "::flush",
        "commandencoder",
    ];
    const RECV: &[&str] = &[
        "try_handle_result",
        "handle_result",
        "receive_result",
        "dispatch_pending",
        "try_match_pubsub",
        "connection::read",
        "try_read",
    ];
    const CALLER: &[&str] = &[
        "internal_send",
        "send_message",
        "prepare_command",
        "stringcommands",
        "into_command",
        "arg_serializer",
        "command_serializer",
    ];

    let any = |needles: &[&str]| frames.iter().any(|f| needles.iter().any(|n| f.contains(n)));

    if any(RECV) {
        Bucket::Receive
    } else if any(SEND) {
        Bucket::Send
    } else if any(CALLER) {
        Bucket::Caller
    } else {
        Bucket::Runtime
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let iters = env_usize("PPROF_ITERS", 200);
    let tasks = env_usize("PPROF_TASKS", 256);
    let reqs = env_usize("PPROF_REQS", 100);
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

    println!("Profiling {iters}×({tasks} tasks × {reqs} GETs) at {hz} Hz (saturated)...");
    for _ in 0..iters {
        fan_out(client.clone(), tasks, reqs, keys.clone()).await;
    }

    let report = guard
        .report()
        .build()
        .expect("failed to build pprof report");

    let out_path = "target/pprof_send_vs_receive_flamegraph.svg";
    let file = std::fs::File::create(out_path).expect("failed to create flamegraph file");
    report
        .flamegraph(file)
        .expect("failed to write flamegraph SVG");
    println!("Flamegraph written to {out_path}\n");

    let mut bucket_totals: HashMap<Bucket, isize> = HashMap::new();
    // Per-bucket hottest leaves, so each half's dominant cost is readable.
    let mut bucket_leaves: HashMap<Bucket, HashMap<String, isize>> = HashMap::new();
    let mut total = 0isize;

    for (frames, count) in report.data.iter() {
        total += *count;
        let stack: Vec<String> = frames
            .frames
            .iter()
            .filter_map(|f| f.first().map(|s| format!("{s}").to_lowercase()))
            .collect();
        if stack.is_empty() {
            continue;
        }
        let bucket = classify(&stack);
        *bucket_totals.entry(bucket).or_default() += *count;
        *bucket_leaves
            .entry(bucket)
            .or_default()
            .entry(stack[0].clone())
            .or_default() += *count;
    }

    println!("=== send vs receive attribution ({total} samples) ===");
    let order = [
        Bucket::Send,
        Bucket::Receive,
        Bucket::Caller,
        Bucket::Runtime,
    ];
    for b in order {
        let c = bucket_totals.get(&b).copied().unwrap_or(0);
        let pct = 100.0 * c as f64 / total.max(1) as f64;
        println!("{pct:6.2}%  {c:>7}  {}", b.label());
    }

    println!("\n=== hottest leaves per bucket ===");
    for b in order {
        let Some(leaves) = bucket_leaves.get(&b) else {
            continue;
        };
        let mut rows: Vec<(&String, &isize)> = leaves.iter().collect();
        rows.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
        println!("\n[{}]", b.label());
        for (sym, count) in rows.iter().take(8) {
            let pct = 100.0 * **count as f64 / total.max(1) as f64;
            println!("  {pct:6.2}%  {count:>7}  {sym}");
        }
    }

    Ok(())
}
