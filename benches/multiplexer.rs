use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;
use tokio::runtime::Runtime;

use fred::prelude::*; // fred
use redis::aio::MultiplexedConnection; // redis-rs
use rustis::{client::Client as RustisClient, commands::StringCommands}; // rustis

async fn setup_data(client: &RustisClient) {
    let data: Vec<_> = (0..100)
        .map(|i| (format!("key{i}"), format!("value{i}")))
        .collect();
    // On s'assure que les données sont bien là avant de commencer
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

async fn bench_fred(client: fred::clients::Client, tasks: usize, reqs: usize, keys: Arc<Vec<String>>) {
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

async fn bench_redis_rs(conn: MultiplexedConnection, tasks: usize, reqs: usize, keys: Arc<Vec<String>>) {
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

    // Initialisation des clients (Setup)
    let (rustis, fred, redis_rs) = rt.block_on(async {
        // rustis
        let rustis = RustisClient::connect("127.0.0.1:6379").await.unwrap();
        setup_data(&rustis).await;
        // fred
        let fred = fred::clients::Client::default();
        fred.init().await.unwrap();
        // redis-rs
        let redis_rs = redis::Client::open("redis://127.0.0.1/").unwrap();
        let redis_rs = redis_rs.get_multiplexed_async_connection().await.unwrap();

        (rustis, fred, redis_rs)
    });

    let mut group = c.benchmark_group("Multiplexing Comparison");
    let num_tasks = 12;
    let reqs_per_task = 200;

    group.bench_function("rustis", |b| {
        b.to_async(&rt)
            .iter(|| bench_rustis(rustis.clone(), num_tasks, reqs_per_task, keys.clone()));
    });

    group.bench_function("fred", |b| {
        b.to_async(&rt)
            .iter(|| bench_fred(fred.clone(), num_tasks, reqs_per_task, keys.clone()));
    });

    group.bench_function("redis-rs", |b| {
        b.to_async(&rt)
            .iter(|| bench_redis_rs(redis_rs.clone(), num_tasks, reqs_per_task, keys.clone()));
    });

    group.finish();
}

criterion_group!(benches, compare_drivers);
criterion_main!(benches);
