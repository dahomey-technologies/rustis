use criterion::{Criterion, criterion_group, criterion_main};
use futures_util::future::join_all;
use rustis::client::Client;
use rustis::resp::{Command, FastPathCommandBuilder, cmd};
use std::hint::black_box;

const KEY: &str = "user:123456789:session";

fn slow_path_get(key: &str) -> Command {
    cmd("GET").key(key).into()
}

fn fast_path_get(key: &str) -> Command {
    FastPathCommandBuilder::get(key)
}

/// A heavy generic command: `HSET key f0 v0 … f19 v19` (41 args). No fast-path
/// builder covers it, so it measures how far generic construction cost grows
/// with argument count — the ceiling of what any fast path could ever save here.
/// Args are pre-built by the caller so the loop times construction only, not
/// `format!` allocations.
fn heavy_generic_hset(fields: &[(String, String)]) -> Command {
    let mut builder = cmd("HSET").key(KEY);
    for (f, v) in fields {
        builder = builder.arg(f).arg(v);
    }
    builder.into()
}

/// Construction-only microbenchmark: how long it takes to *build* a command,
/// with no network. Establishes the raw per-command construction delta and how
/// it scales with argument count.
fn bench_get_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("Redis GET");
    let key = KEY;

    group.bench_function("Slow Path (Generic)", |b| {
        b.iter(|| black_box(slow_path_get(black_box(key))));
    });

    group.bench_function("Fast Path (Static Header)", |b| {
        b.iter(|| black_box(fast_path_get(black_box(key))));
    });

    let fields: Vec<(String, String)> = (0..20)
        .map(|i| (format!("f{i}"), format!("v{i}")))
        .collect();
    group.bench_function("Heavy Generic (HSET 20 fields)", |b| {
        b.iter(|| black_box(heavy_generic_hset(black_box(&fields))));
    });

    group.finish();
}

fn build_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
}

/// Saturate the multiplexer: `tasks` concurrent tasks each issue `reqs`
/// sequential GETs through the full round-trip. The only difference between the
/// two variants is `build` — fast-path vs generic construction — so the measured
/// gap is exactly what a broader fast-path could recover end-to-end.
async fn saturate(client: &Client, tasks: usize, reqs: usize, build: fn(&str) -> Command) {
    let handles: Vec<_> = (0..tasks)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move {
                for _ in 0..reqs {
                    let _: String = client.send(build(KEY), None).await.unwrap();
                }
            })
        })
        .collect();
    join_all(handles).await;
}

/// End-to-end comparison: fast vs generic GET under saturated pipelining, full
/// round-trip against a live Redis on 127.0.0.1:6379. Answers whether moving more
/// commands onto the fast (static-header) construction path is worth it.
fn bench_get_e2e(c: &mut Criterion) {
    const TASKS: usize = 50;
    const REQS: usize = 20; // 1000 round-trips per iteration

    let rt = build_runtime();
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let client = rt.block_on(async {
        let client = Client::connect(host.as_str()).await.unwrap();
        let _: String = client
            .send(cmd("SET").arg(KEY).arg("v"), None)
            .await
            .unwrap();
        client
    });

    let mut group = c.benchmark_group("Redis GET E2E (saturated)");

    group.bench_function("Generic path", |b| {
        b.iter(|| rt.block_on(saturate(&client, TASKS, REQS, slow_path_get)));
    });

    group.bench_function("Fast path", |b| {
        b.iter(|| rt.block_on(saturate(&client, TASKS, REQS, fast_path_get)));
    });

    group.finish();
}

/// Queue `count` GETs into one pipeline, then send. Construction here is serial
/// and not overlapped with any I/O wait: every command is built before the first
/// byte leaves, so the per-command construction delta adds up instead of hiding
/// behind another task's round trip.
async fn pipelined(client: &Client, count: usize, build: fn(&str) -> Command) {
    let mut pipeline = client.create_pipeline();
    for _ in 0..count {
        pipeline.queue_command(build(KEY));
    }
    let _: Vec<String> = pipeline.execute().await.unwrap();
}

/// The regime the saturated bench cannot see: fast vs generic construction when
/// the caller builds a whole batch up front. Full round-trip against live Redis.
fn bench_get_pipeline(c: &mut Criterion) {
    const COUNT: usize = 10_000;

    let rt = build_runtime();
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let client = rt.block_on(async {
        let client = Client::connect(host.as_str()).await.unwrap();
        let _: String = client
            .send(cmd("SET").arg(KEY).arg("v"), None)
            .await
            .unwrap();
        client
    });

    let mut group = c.benchmark_group("Redis GET pipeline (10k queued)");

    group.bench_function("Generic path", |b| {
        b.iter(|| rt.block_on(pipelined(&client, COUNT, slow_path_get)));
    });

    group.bench_function("Fast path", |b| {
        b.iter(|| rt.block_on(pipelined(&client, COUNT, fast_path_get)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_get_commands,
    bench_get_e2e,
    bench_get_pipeline
);
criterion_main!(benches);
