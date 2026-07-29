//! Fault-injecting TCP proxy for failure-path tests.
//!
//! A [`FaultProxy`] binds an ephemeral local port in front of an upstream
//! address (a real Redis, or a fake server in a hermetic test), accepts client
//! connections, and rewrites the **upstream → client** byte stream through a
//! scripted [`Vec<Action>`]. The **client → upstream** direction is always
//! forwarded verbatim, and once the script is exhausted the proxy forwards both
//! directions transparently.
//!
//! [`FaultProxy::start`] scripts one connection; [`FaultProxy::start_multi`]
//! scripts a sequence of them, which is what makes a sustained outage — a server
//! that keeps failing every reconnection — expressible.
//!
//! This is the one primitive that unlocks the faults the client cannot inflict
//! on itself — truncated frames mid-response, unknown RESP3 tags, unsolicited
//! frames, byte-boundary chunking and per-shard errors. The individual scenario
//! tests attach to those cases; this module provides and self-tests the harness
//! itself.

use std::net::SocketAddr;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

/// A single step applied to the upstream → client byte stream, in order.
#[derive(Debug, Clone)]
#[allow(dead_code)] // each variant is exercised by a scenario, not all in one test
pub(crate) enum Action {
    /// Forward exactly `n` bytes from upstream to the client, unchanged.
    PassThrough(usize),
    /// Read `n` bytes from upstream, flip every bit, then forward them —
    /// producing a corrupt frame mid-response.
    Corrupt(usize),
    /// Write `bytes` to the client without consuming any upstream bytes, e.g. an
    /// unsolicited or synthesized frame.
    Inject(Vec<u8>),
    /// Close the client-facing write half, truncating the response mid-stream.
    Truncate,
    /// Pause for `duration` before continuing the script.
    Delay(Duration),
    /// Drop both connections immediately.
    Drop,
}

/// A running fault-injecting proxy. Dropping it aborts the proxy task.
pub(crate) struct FaultProxy {
    /// Local address the client should connect to instead of the upstream.
    pub addr: SocketAddr,
    connections_accepted: Arc<AtomicUsize>,
    /// Handles of the per-connection tasks, so `Drop` tears down the live
    /// connections and not only the accept loop.
    connections: Arc<Mutex<Vec<JoinHandle<()>>>>,
    handle: JoinHandle<()>,
}

impl FaultProxy {
    /// Binds an ephemeral local port in front of `upstream`, then spawns a task
    /// that accepts one client connection, dials `upstream`, and drives the
    /// response stream through `script`.
    pub(crate) async fn start(
        upstream: impl Into<String>,
        script: Vec<Action>,
    ) -> std::io::Result<Self> {
        Self::start_multi(upstream, vec![script]).await
    }

    /// Same as [`Self::start`], but keeps accepting: connection *n* is driven
    /// through `scripts[n]`, and every connection past the end of the list
    /// replays the last script.
    ///
    /// This is what makes a *sustained* outage scriptable rather than a single
    /// failure. With one accept only, the port goes dead after the first
    /// connection is dropped, so a client that keeps reconnecting hits a closed
    /// port instead of a server that keeps failing it — a different fault.
    pub(crate) async fn start_multi(
        upstream: impl Into<String>,
        scripts: Vec<Vec<Action>>,
    ) -> std::io::Result<Self> {
        assert!(
            !scripts.is_empty(),
            "a fault proxy needs at least one script"
        );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let upstream = upstream.into();
        let connections_accepted = Arc::new(AtomicUsize::new(0));

        let connections: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::new(Mutex::new(Vec::new()));

        let accepted = Arc::clone(&connections_accepted);
        let spawned = Arc::clone(&connections);
        let handle = tokio::spawn(async move {
            loop {
                let Ok((client, _)) = listener.accept().await else {
                    return;
                };
                let index = accepted.fetch_add(1, Ordering::Relaxed);
                let script = scripts
                    .get(index)
                    .unwrap_or_else(|| {
                        scripts
                            .last()
                            .expect("a fault proxy needs at least one script")
                    })
                    .clone();

                // Each connection is driven on its own task, so a script that
                // parks (a `Delay`, a transparent tail) does not stop the next
                // reconnection from being accepted.
                let upstream = upstream.clone();
                let connection = tokio::spawn(async move {
                    let Ok(server) = TcpStream::connect(&upstream).await else {
                        return;
                    };
                    let _ = run_connection(client, server, script).await;
                });
                if let Ok(mut guard) = spawned.lock() {
                    guard.push(connection);
                }
            }
        });

        Ok(Self {
            addr,
            connections_accepted,
            connections,
            handle,
        })
    }

    /// Number of client connections accepted so far. Lets a test prove a
    /// reconnection storm actually happened instead of assuming it.
    pub(crate) fn connections_accepted(&self) -> usize {
        self.connections_accepted.load(Ordering::Relaxed)
    }
}

impl Drop for FaultProxy {
    fn drop(&mut self) {
        self.handle.abort();
        if let Ok(guard) = self.connections.lock() {
            for connection in guard.iter() {
                connection.abort();
            }
        }
    }
}

async fn run_connection(
    client: TcpStream,
    server: TcpStream,
    script: Vec<Action>,
) -> std::io::Result<()> {
    let (mut client_read, mut client_write) = client.into_split();
    let (mut server_read, mut server_write) = server.into_split();

    // client → upstream: always transparent, runs concurrently so requests keep
    // flowing while the script rewrites the responses.
    let request_pump = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut client_read, &mut server_write).await;
    });

    for action in script {
        match action {
            Action::PassThrough(n) => {
                let mut buf = vec![0u8; n];
                server_read.read_exact(&mut buf).await?;
                client_write.write_all(&buf).await?;
            }
            Action::Corrupt(n) => {
                let mut buf = vec![0u8; n];
                server_read.read_exact(&mut buf).await?;
                for b in &mut buf {
                    *b = !*b;
                }
                client_write.write_all(&buf).await?;
            }
            Action::Inject(bytes) => {
                client_write.write_all(&bytes).await?;
            }
            Action::Delay(duration) => {
                tokio::time::sleep(duration).await;
            }
            Action::Truncate => {
                client_write.shutdown().await?;
                request_pump.abort();
                return Ok(());
            }
            Action::Drop => {
                request_pump.abort();
                return Ok(());
            }
        }
    }

    // Script exhausted: forward the rest of the response transparently.
    let _ = tokio::io::copy(&mut server_read, &mut client_write).await;
    request_pump.abort();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Result,
        client::Client,
        commands::StringCommands,
        tests::{get_default_addr, log_try_init},
    };

    /// Spawns a minimal upstream that reads a request then replies with
    /// `response`, so the proxy mechanism can be tested without a real Redis.
    async fn spawn_fake_upstream(response: Vec<u8>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 64];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(&response).await;
                let _ = sock.shutdown().await;
            }
        });
        addr
    }

    async fn read_all_from(addr: SocketAddr, request: &[u8]) -> Vec<u8> {
        let mut conn = TcpStream::connect(addr).await.unwrap();
        conn.write_all(request).await.unwrap();
        let mut out = Vec::new();
        conn.read_to_end(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn proxy_passes_the_stream_through_unchanged_when_the_script_is_empty() {
        let upstream = spawn_fake_upstream(b"+PONG\r\n".to_vec()).await;
        let proxy = FaultProxy::start(upstream.to_string(), vec![])
            .await
            .unwrap();
        let out = read_all_from(proxy.addr, b"PING\r\n").await;
        assert_eq!(&out, b"+PONG\r\n");
    }

    #[tokio::test]
    async fn proxy_injects_bytes_without_consuming_upstream() {
        let upstream = spawn_fake_upstream(b"+PONG\r\n".to_vec()).await;
        let proxy = FaultProxy::start(
            upstream.to_string(),
            vec![
                Action::Inject(b"+HELLO\r\n".to_vec()),
                Action::PassThrough(7),
            ],
        )
        .await
        .unwrap();
        let out = read_all_from(proxy.addr, b"PING\r\n").await;
        // The injected frame precedes the real reply, which is still forwarded.
        assert_eq!(&out, b"+HELLO\r\n+PONG\r\n");
    }

    #[tokio::test]
    async fn proxy_corrupts_only_the_scripted_prefix() {
        let upstream = spawn_fake_upstream(b"+PONG\r\n".to_vec()).await;
        let proxy = FaultProxy::start(
            upstream.to_string(),
            vec![Action::Corrupt(1), Action::PassThrough(6)],
        )
        .await
        .unwrap();
        let out = read_all_from(proxy.addr, b"PING\r\n").await;
        assert_eq!(out.len(), 7);
        assert_ne!(out[0], b'+'); // first byte was bit-flipped
        assert_eq!(&out[1..], b"PONG\r\n");
    }

    #[tokio::test]
    async fn proxy_truncates_the_response_mid_stream() {
        let upstream = spawn_fake_upstream(b"+PONGEXTRA\r\n".to_vec()).await;
        let proxy = FaultProxy::start(
            upstream.to_string(),
            vec![Action::PassThrough(3), Action::Truncate],
        )
        .await
        .unwrap();
        let out = read_all_from(proxy.addr, b"PING\r\n").await;
        assert_eq!(&out, b"+PO");
    }

    #[tokio::test]
    async fn proxy_drop_closes_the_connection_immediately() {
        let upstream = spawn_fake_upstream(b"+PONG\r\n".to_vec()).await;
        let proxy = FaultProxy::start(upstream.to_string(), vec![Action::Drop])
            .await
            .unwrap();
        let out = read_all_from(proxy.addr, b"PING\r\n").await;
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn proxy_delay_still_delivers_the_response() {
        let upstream = spawn_fake_upstream(b"+PONG\r\n".to_vec()).await;
        let proxy = FaultProxy::start(
            upstream.to_string(),
            vec![
                Action::Delay(Duration::from_millis(50)),
                Action::PassThrough(7),
            ],
        )
        .await
        .unwrap();
        let out = read_all_from(proxy.addr, b"PING\r\n").await;
        assert_eq!(&out, b"+PONG\r\n");
    }

    /// Spawns an upstream that keeps accepting, so a multi-connection script has
    /// something to dial on every reconnection.
    async fn spawn_repeating_fake_upstream(response: Vec<u8>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let response = response.clone();
                tokio::spawn(async move {
                    let mut buf = [0u8; 64];
                    let _ = sock.read(&mut buf).await;
                    let _ = sock.write_all(&response).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        addr
    }

    /// Successive connections must each get their own script, and connections
    /// past the end of the list must replay the last one — that is what lets a
    /// test script an outage that keeps failing every reconnection.
    #[tokio::test]
    async fn proxy_scripts_each_connection_in_turn_then_repeats_the_last() {
        let upstream = spawn_repeating_fake_upstream(b"+PONG\r\n".to_vec()).await;
        let proxy = FaultProxy::start_multi(
            upstream.to_string(),
            vec![
                vec![Action::Inject(b"+ONE\r\n".to_vec()), Action::Drop],
                vec![Action::Inject(b"+TWO\r\n".to_vec()), Action::Drop],
                vec![Action::Inject(b"+LAST\r\n".to_vec()), Action::Drop],
            ],
        )
        .await
        .unwrap();

        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(read_all_from(proxy.addr, b"PING\r\n").await);
        }

        assert_eq!(
            vec![
                b"+ONE\r\n".to_vec(),
                b"+TWO\r\n".to_vec(),
                b"+LAST\r\n".to_vec(),
                b"+LAST\r\n".to_vec(),
            ],
            seen
        );
        assert_eq!(4, proxy.connections_accepted());
    }

    /// End-to-end proof the harness is usable by a real client: a transparent
    /// proxy in front of Redis must be indistinguishable from a direct
    /// connection.
    #[tokio::test]
    async fn a_real_client_round_trips_through_the_transparent_proxy() -> Result<()> {
        log_try_init();
        let proxy = FaultProxy::start(get_default_addr(), vec![]).await.unwrap();

        let client = Client::connect(format!("redis://{}", proxy.addr)).await?;
        client.set("fault_proxy_smoke_key", "value").await?;
        let value: String = client.get("fault_proxy_smoke_key").await?;
        assert_eq!(value, "value");
        client.close().await?;

        Ok(())
    }
}
