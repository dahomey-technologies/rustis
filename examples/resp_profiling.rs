//! CPU profiler for the RESP parse hot path (frame + tape build), isolated from
//! serde deserialization and from network I/O.
//!
//! Uses `pprof` in userspace (SIGPROF sampling) so it needs neither `perf` nor
//! root. It drives [`bench_parse_only`] — parse only, no deserialize, with a
//! recycled tape buffer (the decoder's zero-allocation steady state) — in a tight
//! loop over a large flat collection and a nested reply, then writes a flamegraph
//! SVG and prints the hottest leaf functions (self time) to stdout.
//!
//! Run with:
//!   cargo run --release --features bench --example resp_profiling
//!
//! Output: `target/resp_parse_flamegraph.svg` + a text table on stdout.
//!
//! Env:
//!   PPROF_ITERS  parse iterations per shape to sample (default 150000)
//!   PPROF_HZ     sampling frequency in Hz (default 4000)

use pprof::ProfilerGuardBuilder;
use rustis::resp::{RespTapeMut, bench_parse_only};
use std::{collections::HashMap, fs::File, hint::black_box};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// A flat RESP array of `n` bulk strings, each `elem_len` bytes.
fn build_array(n: usize, elem_len: usize) -> Vec<u8> {
    let elem = "x".repeat(elem_len);
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{n}\r\n").as_bytes());
    for _ in 0..n {
        buf.extend_from_slice(format!("${elem_len}\r\n").as_bytes());
        buf.extend_from_slice(elem.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }
    buf
}

/// A RESP array of `rows` sub-arrays, each holding `cols` bulk strings of
/// `elem_len` bytes — an `FT.AGGREGATE`-shaped nested reply.
fn build_nested(rows: usize, cols: usize, elem_len: usize) -> Vec<u8> {
    let elem = "x".repeat(elem_len);
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("*{rows}\r\n").as_bytes());
    for _ in 0..rows {
        buf.extend_from_slice(format!("*{cols}\r\n").as_bytes());
        for _ in 0..cols {
            buf.extend_from_slice(format!("${elem_len}\r\n").as_bytes());
            buf.extend_from_slice(elem.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
    }
    buf
}

fn main() {
    let iters = env_usize("PPROF_ITERS", 150_000);
    let hz = env_usize("PPROF_HZ", 4_000) as i32;

    let flat = build_array(5_000, 50);
    let nested = build_nested(500, 10, 20);
    let mut tape = RespTapeMut::default();

    let guard = ProfilerGuardBuilder::default()
        .frequency(hz)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("failed to start pprof profiler");

    println!("Profiling {iters} parse iterations per shape at {hz} Hz...");
    for _ in 0..iters {
        bench_parse_only(black_box(&flat), &mut tape);
        bench_parse_only(black_box(&nested), &mut tape);
    }

    let report = guard
        .report()
        .build()
        .expect("failed to build pprof report");

    let out_path = "target/resp_parse_flamegraph.svg";
    let file = File::create(out_path).expect("failed to create flamegraph file");
    report
        .flamegraph(file)
        .expect("failed to write flamegraph SVG");
    println!("Flamegraph written to {out_path}\n");

    // Aggregate samples by innermost (leaf) symbol = self time, and print the
    // hottest functions so the profile is readable without opening the SVG.
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
    for (name, count) in rows.iter().take(30) {
        let pct = 100.0 * *count as f64 / total.max(1) as f64;
        println!("{pct:6.2}%  {count:>7}  {name}");
    }
}
