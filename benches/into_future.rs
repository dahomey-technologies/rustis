//! What the ergonomic API costs over the generic one.
//!
//! `client.get("key").await` goes through `IntoFuture`, `client.send(cmd("GET")…)`
//! does not. `generic_api` and `native_api` each measure one of the two against
//! other drivers, with different argument types and no shared baseline, so
//! nothing in the suite compares them to each other — which is how a
//! `Box::pin` per command survived on the documented path unmeasured.
//!
//! Two groups, because the two questions have different noise floors:
//!
//! * `round_trip` — the honest end-to-end number, but a loopback GET is tens of
//!   microseconds and a heap allocation is tens of nanoseconds, so the
//!   difference sits under the noise. It is here to catch a regression in the
//!   whole path, not to show the allocation.
//! * `prepare` — building the future and dropping it, never polling it. Both
//!   futures are lazy, so nothing reaches the server and what is left is the
//!   construction cost alone: one heap allocation on the boxed shape, none on
//!   the hand-written one.

use criterion::{Bencher, Criterion, criterion_group, criterion_main};
use rustis::{
    Error,
    client::Client,
    commands::StringCommands,
    resp::{Value, cmd},
};
use std::{future::IntoFuture, hint::black_box, time::Duration};

fn current_thread_runtime() -> tokio::runtime::Runtime {
    let mut builder = tokio::runtime::Builder::new_current_thread();
    builder.enable_io();
    builder.enable_time();
    builder.build().unwrap()
}

async fn get_rustis_client() -> Client {
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let client = Client::connect(redis_host).await.unwrap();
    client.set("key", "value").await.unwrap();
    client
}

/// `client.get(…).await` — the form the documentation teaches.
fn bench_ergonomic_round_trip(b: &mut Bencher) {
    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let _: String = client.get("key").await?;
                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

/// `client.send(…).await` — the generic path, an `async fn` all the way down.
fn bench_generic_round_trip(b: &mut Bencher) {
    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        runtime
            .block_on(async {
                let _: String = client.send(cmd("GET").key("key"), None).await?;
                Ok::<_, Error>(())
            })
            .unwrap()
    });
}

/// Build the awaited future and drop it without polling it: the per-command
/// construction cost on its own, with no reply to wait for.
fn bench_ergonomic_prepare(b: &mut Bencher) {
    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        let future = client.get::<String>("key").into_future();
        black_box(&future);
        drop(future);
    });
}

/// The same, on the generic path.
fn bench_generic_prepare(b: &mut Bencher) {
    let runtime = current_thread_runtime();
    let client = runtime.block_on(get_rustis_client());

    b.iter(|| {
        let future = client.send::<Value>(cmd("GET").key("key"), None);
        black_box(&future);
        drop(future);
    });
}

fn bench_into_future(c: &mut Criterion) {
    let mut group = c.benchmark_group("into_future");
    group
        .measurement_time(Duration::from_secs(10))
        .bench_function("ergonomic_round_trip", bench_ergonomic_round_trip)
        .bench_function("generic_round_trip", bench_generic_round_trip)
        .bench_function("ergonomic_prepare", bench_ergonomic_prepare)
        .bench_function("generic_prepare", bench_generic_prepare);
    group.finish();
}

criterion_group!(bench, bench_into_future);
criterion_main!(bench);
