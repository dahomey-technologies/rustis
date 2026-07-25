//! CPU profiler for command *construction* (no network, no deserialize),
//! comparing the generic builder path against the static-header fast path.
//!
//! Uses `pprof` in userspace (SIGPROF sampling) so it needs neither `perf` nor
//! root. It profiles each path in its own tight loop, writes a flamegraph SVG per
//! path, and prints the hottest leaf functions (self time) so the bottleneck of
//! each path is readable without opening the SVG.
//!
//! Run with:
//!   cargo run --release --features bench --example command_construction_profiling
//!
//! Env:
//!   PPROF_ITERS  construction iterations per path (default 20000000)
//!   PPROF_HZ     sampling frequency in Hz (default 4000)

use pprof::ProfilerGuardBuilder;
use rustis::resp::{Command, FastPathCommandBuilder, cmd};
use std::{collections::HashMap, fs::File, hint::black_box};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn generic_get(key: &str) -> Command {
    cmd("GET").key(key).into()
}

fn fast_get(key: &str) -> Command {
    FastPathCommandBuilder::get(key)
}

fn profile(name: &str, svg: &str, iters: usize, hz: i32, build: fn(&str) -> Command) {
    let key = "user:123456789:session";

    let guard = ProfilerGuardBuilder::default()
        .frequency(hz)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("failed to start pprof profiler");

    println!("Profiling {iters} `{name}` constructions at {hz} Hz...");
    for _ in 0..iters {
        black_box(build(black_box(key)));
    }

    let report = guard
        .report()
        .build()
        .expect("failed to build pprof report");

    let file = File::create(svg).expect("failed to create flamegraph file");
    report.flamegraph(file).expect("failed to write flamegraph");
    println!("Flamegraph written to {svg}");

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

    println!("=== hottest leaf functions — `{name}` self time ({total} samples) ===");
    for (sym, count) in rows.iter().take(25) {
        let pct = 100.0 * *count as f64 / total.max(1) as f64;
        println!("{pct:6.2}%  {count:>7}  {sym}");
    }
    println!();
}

fn main() {
    let iters = env_usize("PPROF_ITERS", 20_000_000);
    let hz = env_usize("PPROF_HZ", 4_000) as i32;

    profile(
        "generic GET",
        "target/construction_generic_flamegraph.svg",
        iters,
        hz,
        generic_get,
    );
    profile(
        "fast GET",
        "target/construction_fast_flamegraph.svg",
        iters,
        hz,
        fast_get,
    );
}
