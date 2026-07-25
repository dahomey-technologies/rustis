use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::sync::Arc;
use tokio::runtime::Runtime;

use fred::prelude::*; // fred
use redis::aio::MultiplexedConnection; // redis-rs
use rustis::{client::Client as RustisClient, commands::StringCommands}; // rustis

/// Concurrency levels that saturate the shared connection. The latency-bound
/// `multiplexer` bench (12 tasks) keeps the pipe half-empty; here we push it
/// until the single network task becomes the throughput ceiling, which is where
/// write-coalescing (commands per `writev`) actually decides the winner.
const CONCURRENCY: &[usize] = &[64, 256, 1024];
const REQS_PER_TASK: usize = 100;

async fn setup_data(client: &RustisClient) {
    let data: Vec<_> = (0..100)
        .map(|i| (format!("key{i}"), format!("value{i}")))
        .collect();
    let _: () = client.mset(data).await.unwrap();
}

async fn bench_rustis(client: RustisClient, tasks: usize, reqs: usize, keys: Arc<Vec<String>>) {
    let mut handles = vec![];
    for _ in 0..tasks {
        let client = client.clone();
        let keys = keys.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..reqs {
                let _: String = rustis::commands::StringCommands::get(&client, &keys[i % 100])
                    .await
                    .unwrap();
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}

async fn bench_fred(
    client: fred::clients::Client,
    tasks: usize,
    reqs: usize,
    keys: Arc<Vec<String>>,
) {
    let mut handles = vec![];
    for _ in 0..tasks {
        let client = client.clone();
        let keys = keys.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..reqs {
                let _: String = client.get(&keys[i % 100]).await.unwrap();
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}

async fn bench_redis_rs(
    conn: MultiplexedConnection,
    tasks: usize,
    reqs: usize,
    keys: Arc<Vec<String>>,
) {
    let mut handles = vec![];
    for _ in 0..tasks {
        let mut conn = conn.clone();
        let keys = keys.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..reqs {
                let _: String = redis::cmd("GET")
                    .arg(&keys[i % 100])
                    .query_async(&mut conn)
                    .await
                    .unwrap();
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}

fn compare_drivers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let keys: Arc<Vec<String>> = Arc::new((0..100).map(|i| format!("key{i}")).collect());
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let (rustis, fred, redis_rs) = rt.block_on(async {
        // rustis
        let rustis = RustisClient::connect(redis_host.clone()).await.unwrap();
        setup_data(&rustis).await;
        // fred
        let config = Config::from_url(&format!("redis://{redis_host}:6379/0")).unwrap();
        let fred = Builder::from_config(config).build().unwrap();
        fred.init().await.unwrap();
        // redis-rs
        let redis_rs = redis::Client::open(format!("redis://{redis_host}:6379")).unwrap();
        let redis_rs = redis_rs.get_multiplexed_async_connection().await.unwrap();

        (rustis, fred, redis_rs)
    });

    for &tasks in CONCURRENCY {
        let mut group = c.benchmark_group(format!("Saturated Throughput ({tasks} tasks)"));
        // Report ops/s: total GETs issued per iteration.
        group.throughput(Throughput::Elements((tasks * REQS_PER_TASK) as u64));

        group.bench_function("rustis", |b| {
            b.to_async(&rt)
                .iter(|| bench_rustis(rustis.clone(), tasks, REQS_PER_TASK, keys.clone()));
        });

        group.bench_function("fred", |b| {
            b.to_async(&rt)
                .iter(|| bench_fred(fred.clone(), tasks, REQS_PER_TASK, keys.clone()));
        });

        group.bench_function("redis-rs", |b| {
            b.to_async(&rt)
                .iter(|| bench_redis_rs(redis_rs.clone(), tasks, REQS_PER_TASK, keys.clone()));
        });

        group.finish();
    }
}

criterion_group!(benches, compare_drivers);
criterion_main!(benches);
