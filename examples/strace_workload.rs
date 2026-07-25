//! Fixed-size fan-out GET workload runnable against either rustis or redis-rs,
//! for black-box syscall counting under an LD_PRELOAD write() shim. Same shape
//! as the `Multiplexing Comparison` benchmark so the write-coalescing behaviour
//! of the two drivers can be compared apples-to-apples.
//!
//!   DRIVER=rustis|redis  cargo run --release --features bench --example strace_workload
//!
//! Env: REDIS_HOST (default 127.0.0.1), WK_TASKS (default 12), WK_REQS (default 2000).

use std::sync::Arc;

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[tokio::main]
async fn main() {
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let driver = std::env::var("DRIVER").unwrap_or_else(|_| "rustis".to_string());
    let tasks = env_usize("WK_TASKS", 12);
    let reqs = env_usize("WK_REQS", 2000);
    let keys: Arc<Vec<String>> = Arc::new((0..100).map(|i| format!("key{i}")).collect());
    let total = tasks * reqs;

    match driver.as_str() {
        "rustis" => {
            use rustis::{client::Client, commands::StringCommands};
            let client = Client::connect(host).await.unwrap();
            let data: Vec<_> = (0..100)
                .map(|i| (format!("key{i}"), format!("value{i}")))
                .collect();
            let _: () = client.mset(data).await.unwrap();
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
        "redis" => {
            let client = redis::Client::open(format!("redis://{host}:6379")).unwrap();
            let conn = client.get_multiplexed_async_connection().await.unwrap();
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
        other => panic!("unknown DRIVER: {other}"),
    }

    eprintln!("[workload] driver={driver} total_requests={total}");
}
