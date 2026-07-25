//! Isolates the per-request cross-task *wake-up* cost paid by the multiplexer
//! architecture, independent of any Redis round-trip.
//!
//! The multiplexer routes every request through a dedicated network task: the
//! caller sends a message on a `futures_channel::mpsc` (waking the network
//! task) and awaits its reply on a per-request `oneshot` (waking the caller
//! back). That is two scheduler hand-offs per request. This probe reproduces
//! exactly those primitives with a no-op "server" (the network task replies
//! immediately), so the measured time is purely: mpsc send + task wake-up +
//! oneshot send + caller wake-up. No socket, no kernel I/O, no RTT.
//!
//! Phase A: one in-flight request at a time (latency of a single coordination
//!          round-trip). Phase B: `TASKS` concurrent callers hammering the one
//!          consumer, matching the benchmark's fan-out (contention + batching
//!          on the drain side). Phase C (control): the same two hand-offs but
//!          with a real localhost TCP echo in between, to show how much the
//!          socket RTT dwarfs the channel coordination.
//!
//! Run with:
//!   cargo run --release --features bench --example wakeup_cost_probe
//!
//! Env:
//!   WAKE_ITERS  measured iterations per phase (default 200_000)
//!   WAKE_TASKS  concurrent callers in phase B (default 12)

use futures_channel::{mpsc, oneshot};
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

type Reply = oneshot::Sender<()>;

fn summarize(name: &str, mut samples: Vec<Duration>) {
    samples.sort_unstable();
    let n = samples.len();
    let sum: Duration = samples.iter().sum();
    let mean = sum / n as u32;
    let pct = |p: f64| samples[((n as f64 * p) as usize).min(n - 1)];
    println!(
        "{name:<28} n={n:>8}  mean={:>7.3}µs  p50={:>7.3}µs  p99={:>7.3}µs  max={:>7.3}µs",
        mean.as_secs_f64() * 1e6,
        pct(0.50).as_secs_f64() * 1e6,
        pct(0.99).as_secs_f64() * 1e6,
        samples[n - 1].as_secs_f64() * 1e6,
    );
}

#[tokio::main]
async fn main() {
    let iters = env_usize("WAKE_ITERS", 200_000);
    let tasks = env_usize("WAKE_TASKS", 12);

    // ---- Phases A & B: pure coordination, no-op server task ----
    let (tx, mut rx) = mpsc::unbounded::<Reply>();
    let server = tokio::spawn(async move {
        // Drain like the network loop does: take one, then greedily take all
        // currently-ready messages before yielding, replying to each.
        while let Some(reply) = rx.next().await {
            let _ = reply.send(());
            while let Ok(reply) = rx.try_recv() {
                let _ = reply.send(());
            }
        }
    });

    // Phase A: single in-flight request at a time.
    {
        let mut samples = Vec::with_capacity(iters);
        // warm-up
        for _ in 0..1000 {
            let (rtx, rrx) = oneshot::channel();
            tx.unbounded_send(rtx).unwrap();
            let _ = rrx.await;
        }
        for _ in 0..iters {
            let (rtx, rrx) = oneshot::channel();
            let t0 = Instant::now();
            tx.unbounded_send(rtx).unwrap();
            let _ = rrx.await;
            samples.push(t0.elapsed());
        }
        summarize("A: 1-inflight round-trip", samples);
    }

    // Phase B: `tasks` concurrent callers, each timing its own round-trips.
    {
        let per_task = iters / tasks;
        let mut handles = Vec::with_capacity(tasks);
        for _ in 0..tasks {
            let tx = tx.clone();
            handles.push(tokio::spawn(async move {
                let mut samples = Vec::with_capacity(per_task);
                for _ in 0..per_task {
                    let (rtx, rrx) = oneshot::channel();
                    let t0 = Instant::now();
                    tx.unbounded_send(rtx).unwrap();
                    let _ = rrx.await;
                    samples.push(t0.elapsed());
                }
                samples
            }));
        }
        let mut all = Vec::with_capacity(per_task * tasks);
        for h in handles {
            all.extend(h.await.unwrap());
        }
        summarize(&format!("B: {tasks}-task contended"), all);
    }

    drop(tx);
    let _ = server.await;

    // ---- Phase C (control): same two hand-offs + real localhost TCP echo ----
    // A caller task sends a byte to an echo server over loopback and awaits the
    // reply. This is the minimal socket round-trip that a real GET cannot beat,
    // so it bounds how much the channel coordination above actually matters.
    {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let echo = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 64];
            loop {
                match sock.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if sock.write_all(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        sock.set_nodelay(true).unwrap();
        let mut buf = [0u8; 64];
        let ctrl_iters = (iters / 4).max(1);
        for _ in 0..1000 {
            sock.write_all(b"x").await.unwrap();
            let _ = sock.read(&mut buf).await.unwrap();
        }
        let mut samples = Vec::with_capacity(ctrl_iters);
        for _ in 0..ctrl_iters {
            let t0 = Instant::now();
            sock.write_all(b"x").await.unwrap();
            let _ = sock.read(&mut buf).await.unwrap();
            samples.push(t0.elapsed());
        }
        summarize("C: loopback TCP echo", samples);
        drop(sock);
        let _ = echo.await;
    }
}
