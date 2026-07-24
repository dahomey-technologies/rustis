//! CPU profiler for the 1000-command pipeline hot path.
//!
//! Uses `pprof` in userspace (SIGPROF sampling) so it needs neither `perf`
//! nor root — unlike a kernel flamegraph. It drives the exact workload of the
//! `rustis_long_pipeline` benchmark in a tight loop under a sampling guard, then
//! writes a flamegraph SVG.
//!
//! Run with:
//!   cargo run --release --features bench --example pprof_pipeline
//!
//! Output: `target/pprof_pipeline_flamegraph.svg`
//!
//! Env:
//!   REDIS_HOST   Redis host (default 127.0.0.1)
//!   PPROF_ITERS  number of pipeline round-trips to sample (default 5000)
//!   PPROF_HZ     sampling frequency in Hz (default 1000)

use rustis::{
    Result,
    client::{BatchPreparedCommand, Client},
    commands::StringCommands,
};

const PIPELINE_QUERIES: usize = 1_000;

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[tokio::main]
async fn main() -> Result<()> {
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let iters = env_usize("PPROF_ITERS", 5_000);
    let hz = env_usize("PPROF_HZ", 1_000) as i32;

    let client = Client::connect(redis_host).await?;

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(hz)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("failed to start pprof profiler");

    println!("Profiling {iters} pipelines of {PIPELINE_QUERIES} commands at {hz} Hz...");

    for _ in 0..iters {
        let mut pipeline = client.create_pipeline();
        pipeline.reserve(PIPELINE_QUERIES);

        for i in 0..PIPELINE_QUERIES {
            pipeline.set(format!("foo{i}"), "bar").queue();
        }

        let _result: Vec<String> = pipeline.execute().await?;
    }

    let report = guard
        .report()
        .build()
        .expect("failed to build pprof report");
    let out_path = "target/pprof_pipeline_flamegraph.svg";
    let file = std::fs::File::create(out_path).expect("failed to create flamegraph file");
    report
        .flamegraph(file)
        .expect("failed to write flamegraph SVG");

    println!("Flamegraph written to {out_path}");
    Ok(())
}
