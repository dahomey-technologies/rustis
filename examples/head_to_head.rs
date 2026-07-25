//! Fast, low-ceremony head-to-head throughput probe: rustis vs redis-rs vs fred
//! on the same runtime, same keys, same concurrency, interleaved round-robin so
//! machine drift hits every driver equally.
//!
//! Criterion is too slow to iterate on the serial-path work; this reports, per
//! concurrency level, the median ops/s of each driver plus the median of the
//! *per-round* ratio to redis-rs. Pairing the rounds cancels the machine drift
//! that a best-of-N comparison leaves in.
//!
//!   cargo run --release --features bench --example head_to_head
//!
//! Env:
//!   REDIS_HOST  Redis host (default 127.0.0.1)
//!   HH_TASKS    concurrent tasks, comma separated (default 64,256,1024)
//!   HH_REQS     sequential GETs per task (default 400)
//!   HH_ROUNDS   measured rounds per driver (default 5)
//!   HH_DRIVERS  comma separated subset of rustis,redis,fred (default all)

use std::{sync::Arc, time::Instant};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_list(key: &str, default: &str) -> Vec<String> {
    std::env::var(key)
        .unwrap_or_else(|_| default.to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

async fn run_rustis(
    client: rustis::client::Client,
    tasks: usize,
    reqs: usize,
    keys: Arc<Vec<String>>,
) {
    use rustis::commands::StringCommands;
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

async fn run_redis(
    conn: redis::aio::MultiplexedConnection,
    tasks: usize,
    reqs: usize,
    keys: Arc<Vec<String>>,
) {
    let mut handles = Vec::with_capacity(tasks);
    for _ in 0..tasks {
        let mut conn = conn.clone();
        let keys = keys.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..reqs {
                let _: String = redis::cmd("GET")
                    .arg(&keys[i % keys.len()])
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

async fn run_fred(
    client: fred::clients::Client,
    tasks: usize,
    reqs: usize,
    keys: Arc<Vec<String>>,
) {
    use fred::interfaces::KeysInterface;
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

#[tokio::main]
async fn main() -> rustis::Result<()> {
    let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let task_levels: Vec<usize> = env_list("HH_TASKS", "64,256,1024")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let reqs = env_usize("HH_REQS", 400);
    let rounds = env_usize("HH_ROUNDS", 5);
    let drivers = env_list("HH_DRIVERS", "rustis,redis,fred");

    let keys: Arc<Vec<String>> = Arc::new((0..100).map(|i| format!("key{i}")).collect());

    let rustis_client = rustis::client::Client::connect(host.clone()).await?;
    {
        use rustis::commands::StringCommands;
        let data: Vec<_> = (0..100)
            .map(|i| (format!("key{i}"), format!("value{i}")))
            .collect();
        let _: () = rustis_client.mset(data).await?;
    }

    let redis_conn = if drivers.iter().any(|d| d == "redis") {
        let c = redis::Client::open(format!("redis://{host}:6379")).unwrap();
        Some(c.get_multiplexed_async_connection().await.unwrap())
    } else {
        None
    };

    let fred_client = if drivers.iter().any(|d| d == "fred") {
        use fred::prelude::*;
        let config = Config::from_url(&format!("redis://{host}:6379/0")).unwrap();
        let c = Builder::from_config(config).build().unwrap();
        c.init().await.unwrap();
        Some(c)
    } else {
        None
    };

    for &tasks in &task_levels {
        let total = (tasks * reqs) as f64;
        // One measurement per driver per round, always in the same order, so a
        // drift in machine state hits every driver within a few milliseconds of
        // each other. Ratios are then computed *per round* and the median ratio
        // is reported: paired comparison cancels the drift that best-of-N does
        // not.
        let mut per_round: Vec<Vec<f64>> = vec![Vec::with_capacity(rounds); drivers.len()];

        for round in 0..rounds + 1 {
            for (idx, name) in drivers.iter().enumerate() {
                let start = Instant::now();
                match name.as_str() {
                    "rustis" => {
                        run_rustis(rustis_client.clone(), tasks, reqs, keys.clone()).await;
                    }
                    "redis" => {
                        run_redis(
                            redis_conn.clone().expect("redis connection"),
                            tasks,
                            reqs,
                            keys.clone(),
                        )
                        .await;
                    }
                    "fred" => {
                        run_fred(
                            fred_client.clone().expect("fred client"),
                            tasks,
                            reqs,
                            keys.clone(),
                        )
                        .await;
                    }
                    other => panic!("unknown driver: {other}"),
                }
                if round > 0 {
                    per_round[idx].push(total / start.elapsed().as_secs_f64());
                }
            }
        }

        let median = |v: &mut Vec<f64>| {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[v.len() / 2]
        };

        let reference_idx = drivers.iter().position(|n| n == "redis");
        println!("--- {tasks} tasks × {reqs} GETs ({rounds} paired rounds) ---");
        for (idx, name) in drivers.iter().enumerate() {
            let mut samples = per_round[idx].clone();
            let med = median(&mut samples);
            let lo = samples[0];
            let hi = samples[samples.len() - 1];
            match reference_idx {
                Some(r) if r != idx => {
                    let mut ratios: Vec<f64> = per_round[idx]
                        .iter()
                        .zip(per_round[r].iter())
                        .map(|(a, b)| a / b)
                        .collect();
                    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let med_ratio = 100.0 * (ratios[ratios.len() / 2] - 1.0);
                    let worst = 100.0 * (ratios[0] - 1.0);
                    let best = 100.0 * (ratios[ratios.len() - 1] - 1.0);
                    println!(
                        "{name:>8}  med {med:>9.0}  [{lo:.0}..{hi:.0}]  vs redis-rs {med_ratio:+6.1}% [{worst:+.1}..{best:+.1}]"
                    );
                }
                _ => println!("{name:>8}  med {med:>9.0}  [{lo:.0}..{hi:.0}]"),
            }
        }
        println!();
    }

    Ok(())
}
