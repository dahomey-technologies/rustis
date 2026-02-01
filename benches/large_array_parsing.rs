use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use tokio::runtime::Runtime;

use fred::prelude::*; // fred
use redis::{AsyncCommands, aio::MultiplexedConnection}; // redis-rs
use rustis::{
    client::Client as RustisClient,
    commands::{GenericCommands, ListCommands},
}; // rustis

const KEY: &str = "large_list";

async fn setup_data(client: &RustisClient) {
    // Cleanup
    let _: usize = client.del(KEY).await.unwrap();

    // prepare 500 strings of around 100 bytes each to force parser to work on volume
    let payloads: Vec<String> = (0..5000)
        .map(|i| {
            format!(
                "user_session_data_buffer_overflow_protection_check_{:05}",
                i
            )
        })
        .collect();

    client.rpush(KEY, payloads).await.unwrap();
}

async fn bench_rustis(client: RustisClient) {
    let _: Vec<String> = black_box(client.lrange(KEY, 0, -1).await.unwrap());
}

async fn bench_fred(client: fred::clients::Client) {
    let _: Vec<String> = black_box(client.lrange(KEY, 0, -1).await.unwrap());
}

async fn bench_redis_rs(mut conn: MultiplexedConnection) {
    let _: Vec<String> = black_box(conn.lrange(KEY, 0, -1).await.unwrap());
}

fn compare_drivers(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let redis_host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    // Initialisation des clients (Setup)
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

    let mut group = c.benchmark_group("Large Array Parsing Comparison");

    group.bench_function("rustis", |b| {
        b.to_async(&rt).iter(|| bench_rustis(rustis.clone()));
    });

    group.bench_function("fred", |b| {
        b.to_async(&rt).iter(|| bench_fred(fred.clone()));
    });

    group.bench_function("redis-rs", |b| {
        b.to_async(&rt).iter(|| bench_redis_rs(redis_rs.clone()));
    });

    group.finish();
}

criterion_group!(benches, compare_drivers);
criterion_main!(benches);
