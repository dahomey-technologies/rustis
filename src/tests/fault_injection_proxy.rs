//! Fault-injecting TCP proxy for failure-path tests.
//!
//! A [`FaultProxy`] binds an ephemeral local port in front of an upstream
//! address (a real Redis, or a fake server in a hermetic test), accepts a
//! single client connection, and rewrites the **upstream → client** byte stream
//! through a scripted [`Vec<Action>`]. The **client → upstream** direction is
//! always forwarded verbatim, and once the script is exhausted the proxy
//! forwards both directions transparently.
//!
//! This is the one primitive that unlocks the faults the client cannot inflict
//! on itself — truncated frames mid-response, unknown RESP3 tags, unsolicited
//! frames, byte-boundary chunking and per-shard errors. The individual scenario
//! tests attach to those cases; this module provides and self-tests the harness
//! itself.

use std::net::SocketAddr;
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
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let upstream = upstream.into();

        let handle = tokio::spawn(async move {
            let Ok((client, _)) = listener.accept().await else {
                return;
            };
            let Ok(server) = TcpStream::connect(&upstream).await else {
                return;
            };
            let _ = run_connection(client, server, script).await;
        });

        Ok(Self { addr, handle })
    }
}

impl Drop for FaultProxy {
    fn drop(&mut self) {
        self.handle.abort();
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
