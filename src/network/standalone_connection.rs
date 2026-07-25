use crate::{
    Error, Future, Result, RetryReason, TcpStreamReader, TcpStreamWriter,
    client::{Config, PreparedCommand},
    commands::{
        ClusterCommands, ConnectionCommands, HelloOptions, SentinelCommands, ServerCommands,
    },
    resp::{BufferDecoder, Command, CommandEncoder, RespResponse},
    tcp_connect,
};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::{TcpTlsStreamReader, TcpTlsStreamWriter, tcp_tls_connect};
use bytes::BytesMut;
use futures_util::{SinkExt, Stream, StreamExt, task::noop_waker_ref};
use log::{Level, debug, log_enabled, trace};
use serde::de::DeserializeOwned;
use std::{
    future::IntoFuture,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio_util::codec::{FramedRead, FramedWrite};

/// Initial capacity of the read framing buffer, and the target both the read
/// and write buffers are shrunk back to once they have grown oversized.
/// `FramedRead` otherwise starts at tokio-util's 8 KiB default; a 64 KiB start
/// matches the shrink target so the two are one parameter (a knob that could
/// later move to `Config`).
pub(crate) const TARGET_BUFFER_CAPACITY: usize = 64 * 1024;

/// A buffer is only considered for shrinking once its capacity exceeds this
/// multiple of the target, so a workload alternating large and small replies
/// does not reallocate every cycle (hysteresis, part 1).
const BUFFER_SHRINK_FACTOR: usize = 8;

/// Consecutive under-target observations required before actually paying for
/// the shrink realloc (hysteresis, part 2).
const BUFFER_SHRINK_HYSTERESIS: usize = 16;

/// Replaces `buf` with a fresh `TARGET_BUFFER_CAPACITY` buffer once it has been
/// oversized and near-empty for long enough, returning its high-water-mark
/// memory to the allocator. `BytesMut` has no `shrink_to_fit`, so replacement
/// is the only lever.
///
/// `small_streak` is the caller-owned hysteresis counter for this buffer.
fn maybe_shrink_buffer(buf: &mut BytesMut, small_streak: &mut usize) {
    // Part 1: ignore buffers that have not grown well past the target.
    if buf.capacity() <= TARGET_BUFFER_CAPACITY * BUFFER_SHRINK_FACTOR {
        *small_streak = 0;
        return;
    }
    // The residue must fit the fresh buffer for the copy below to stay within
    // the target; if it does not, the buffer is legitimately busy right now.
    if buf.len() > TARGET_BUFFER_CAPACITY {
        *small_streak = 0;
        return;
    }
    // Part 2: require a streak of quiet observations before reallocating.
    *small_streak += 1;
    if *small_streak < BUFFER_SHRINK_HYSTERESIS {
        return;
    }
    *small_streak = 0;
    let mut replacement = BytesMut::with_capacity(TARGET_BUFFER_CAPACITY);
    replacement.extend_from_slice(buf);
    *buf = replacement;
}

pub(crate) enum Streams {
    Tcp(
        FramedRead<TcpStreamReader, BufferDecoder>,
        FramedWrite<TcpStreamWriter, CommandEncoder>,
    ),
    #[cfg(any(feature = "native-tls", feature = "rustls"))]
    TcpTls(
        FramedRead<TcpTlsStreamReader, BufferDecoder>,
        FramedWrite<TcpTlsStreamWriter, CommandEncoder>,
    ),
}

impl Streams {
    pub async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        if let Some(tls_config) = &config.tls_config {
            let (reader, writer) =
                tcp_tls_connect(host, port, tls_config, config.connect_timeout).await?;
            let framed_read =
                FramedRead::with_capacity(reader, BufferDecoder::new(), TARGET_BUFFER_CAPACITY);
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            Self::connect_non_secure(host, port, config).await
        }

        #[cfg(not(any(feature = "native-tls", feature = "rustls")))]
        Self::connect_non_secure(host, port, config).await
    }

    pub async fn connect_non_secure(host: &str, port: u16, config: &Config) -> Result<Self> {
        let (reader, writer) = tcp_connect(host, port, config).await?;
        let framed_read = FramedRead::new(reader, BufferDecoder::new());
        let framed_write = FramedWrite::new(writer, CommandEncoder);
        Ok(Streams::Tcp(framed_read, framed_write))
    }
}

pub struct StandaloneConnection {
    host: String,
    port: u16,
    config: Config,
    streams: Streams,
    version: String,
    tag: Arc<str>,
    /// Hysteresis counter for the read buffer's shrink policy.
    read_buffer_small_streak: usize,
    /// Hysteresis counter for the write buffer's shrink policy.
    write_buffer_small_streak: usize,
    /// Test-only: number of read attempts remaining before the connection
    /// simulates being closed (see [`Command::kill_connection_on_read`]).
    #[cfg(test)]
    kill_connection_on_read_countdown: usize,
}

impl StandaloneConnection {
    pub async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        let streams = Streams::connect(host, port, config).await?;

        let mut connection = Self {
            host: host.to_owned(),
            port,
            config: config.clone(),
            streams,
            version: String::new(),
            tag: if config.connection_name.is_empty() {
                format!("{host}:{port}").into()
            } else {
                format!("{}:{}:{}", config.connection_name, host, port).into()
            },
            read_buffer_small_streak: 0,
            write_buffer_small_streak: 0,
            #[cfg(test)]
            kill_connection_on_read_countdown: 0,
        };

        connection.post_connect().await?;

        Ok(connection)
    }

    /// Returns the oversized read/write buffers to the allocator once they have
    /// been quiet long enough. Disjoint field borrows let the streak counters and
    /// `streams` be mutated together.
    fn shrink_read_buffer(&mut self) {
        let streak = &mut self.read_buffer_small_streak;
        match &mut self.streams {
            Streams::Tcp(framed_read, _) => {
                maybe_shrink_buffer(framed_read.read_buffer_mut(), streak)
            }
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(framed_read, _) => {
                maybe_shrink_buffer(framed_read.read_buffer_mut(), streak)
            }
        }
    }

    fn shrink_write_buffer(&mut self) {
        let streak = &mut self.write_buffer_small_streak;
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => {
                maybe_shrink_buffer(framed_write.write_buffer_mut(), streak)
            }
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => {
                maybe_shrink_buffer(framed_write.write_buffer_mut(), streak)
            }
        }
    }

    async fn write(&mut self, command: &Command) -> Result<()> {
        debug!("[{}] Sending command: {command}", self.tag);
        let result = match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.send(command).await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.send(command).await,
        };
        // `send` flushes, so the write buffer is drained here — a good moment to
        // reclaim it if one oversized command inflated it.
        self.shrink_write_buffer();
        result
    }

    pub async fn feed(&mut self, command: &Command, _retry_reasons: &[RetryReason]) -> Result<()> {
        debug!("[{}] Sending command: {command}", self.tag);

        #[cfg(test)]
        if command.try_decrement_kill_connection_on_write() {
            let client_id = self.client_id().await?;
            let mut config = self.config.clone();
            "killer".clone_into(&mut config.connection_name);
            let mut connection =
                StandaloneConnection::connect(&self.host, self.port, &config).await?;
            connection
                .client_kill(crate::commands::ClientKillOptions::default().id(client_id))
                .await?;
        }

        // Test-only: arm the read-kill countdown once, when the marked command
        // is fed. `swap` makes it one-shot so a replayed command cannot re-arm.
        #[cfg(test)]
        {
            let num_reads = command
                .kill_connection_on_read
                .swap(0, std::sync::atomic::Ordering::SeqCst);
            if num_reads > 0 {
                self.kill_connection_on_read_countdown = num_reads;
            }
        }

        match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.feed(command).await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.feed(command).await,
        }
    }

    pub async fn flush(&mut self) -> Result<()> {
        trace!("[{}] Flushing...", self.tag);
        let result = match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.flush().await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.flush().await,
        };
        // The write buffer is now drained; reclaim it if it grew oversized.
        self.shrink_write_buffer();
        result
    }

    pub async fn read(&mut self) -> Option<Result<RespResponse>> {
        // Test-only: simulate the connection being closed before any response
        // is delivered, once the armed countdown expires.
        #[cfg(test)]
        if self.kill_connection_on_read_countdown > 0 {
            self.kill_connection_on_read_countdown -= 1;
            if self.kill_connection_on_read_countdown == 0 {
                debug!("[{}] Simulating a closed socket on read", self.tag);
                return None;
            }
        }

        let next = match &mut self.streams {
            Streams::Tcp(framed_read, _) => framed_read.next().await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(framed_read, _) => framed_read.next().await,
        };

        // Reclaim the read buffer if a large reply left it oversized and it has
        // since drained back below the target for long enough.
        self.shrink_read_buffer();

        if let Some(result) = next {
            if log_enabled!(Level::Debug) {
                match &result {
                    Ok(response) => debug!("[{}] Received response {response:?}", self.tag),
                    Err(err) => debug!("[{}] Received response {err:?}", self.tag),
                }
            }
            Some(result)
        } else {
            debug!("[{}] Socked is closed", self.tag);
            None
        }
    }

    pub fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        // Test-only: mirror `read`'s simulated close on the drain path.
        #[cfg(test)]
        if self.kill_connection_on_read_countdown > 0 {
            self.kill_connection_on_read_countdown -= 1;
            if self.kill_connection_on_read_countdown == 0 {
                debug!(
                    "[{}] (try_read) Simulating a closed socket on read",
                    self.tag
                );
                return Poll::Ready(None);
            }
        }

        let waker = noop_waker_ref();
        let mut cx = Context::from_waker(waker);

        let poll_result = match &mut self.streams {
            Streams::Tcp(framed_read, _) => Pin::new(framed_read).poll_next(&mut cx),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(framed_read, _) => Pin::new(framed_read).poll_next(&mut cx),
        };

        // Same reclaim as the async `read` path; a no-op mid-large-frame because
        // the residue then exceeds the target.
        self.shrink_read_buffer();

        match poll_result {
            Poll::Ready(Some(result)) => {
                if log_enabled!(Level::Debug) {
                    match &result {
                        Ok(response) => {
                            debug!("[{}] (try_read) Received result {response:?}", self.tag)
                        }
                        Err(err) => debug!("[{}] (try_read) Received result {err:?}", self.tag),
                    }
                }
                Poll::Ready(Some(result))
            }
            Poll::Ready(None) => {
                debug!("[{}] Socket is closed", self.tag);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending, // Nothing to read right now
        }
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        self.streams = Streams::connect(&self.host, self.port, &self.config).await?;
        // Fresh streams carry fresh buffers, so the shrink hysteresis restarts.
        self.read_buffer_small_streak = 0;
        self.write_buffer_small_streak = 0;
        self.post_connect().await?;

        Ok(())
    }

    async fn post_connect(&mut self) -> Result<()> {
        // RESP3
        let mut hello_options = HelloOptions::new(3);

        let config_username = self.config.username.clone();
        let config_password = self.config.password.clone();
        let config_connection_name = self.config.connection_name.clone();

        // authentication
        if let Some(password) = &config_password {
            hello_options = hello_options.auth(
                match &config_username {
                    Some(username) => username,
                    None => "default",
                },
                password,
            );
        }

        // connection name
        if !config_connection_name.is_empty() {
            hello_options = hello_options.set_name(&config_connection_name);
        }

        let hello_result = self.hello(hello_options).await?;
        self.version = hello_result.version;

        // select database
        if self.config.database != 0 {
            self.select(self.config.database).await?;
        }

        Ok(())
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub(crate) fn tag(&self) -> Arc<str> {
        self.tag.clone()
    }
}

impl<'a, R> IntoFuture for PreparedCommand<'a, &'a mut StandaloneConnection, R>
where
    R: DeserializeOwned + Send + 'a,
{
    type Output = Result<R>;
    type IntoFuture = Future<'a, R>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            self.executor.write(&self.command).await?;

            let response = self
                .executor
                .read()
                .await
                .ok_or_else(|| Error::DisconnectedByPeer)??;

            response.to()
        })
    }
}

impl<'a> ClusterCommands<'a> for &'a mut StandaloneConnection {}
impl<'a> ConnectionCommands<'a> for &'a mut StandaloneConnection {}
impl<'a> SentinelCommands<'a> for &'a mut StandaloneConnection {}
impl<'a> ServerCommands<'a> for &'a mut StandaloneConnection {}

#[cfg(test)]
mod tests {
    use super::{
        BUFFER_SHRINK_FACTOR, BUFFER_SHRINK_HYSTERESIS, TARGET_BUFFER_CAPACITY, maybe_shrink_buffer,
    };
    use bytes::BytesMut;

    const OVERSIZED: usize = TARGET_BUFFER_CAPACITY * BUFFER_SHRINK_FACTOR + 1;

    #[test]
    fn does_not_shrink_a_buffer_within_the_factor() {
        // A buffer that never grew past factor × target is left alone and never
        // accrues a streak.
        let mut buf = BytesMut::with_capacity(TARGET_BUFFER_CAPACITY);
        let mut streak = 0;
        for _ in 0..BUFFER_SHRINK_HYSTERESIS * 2 {
            maybe_shrink_buffer(&mut buf, &mut streak);
        }
        assert_eq!(streak, 0);
        assert_eq!(buf.capacity(), TARGET_BUFFER_CAPACITY);
    }

    #[test]
    fn shrinks_an_oversized_idle_buffer_only_after_the_hysteresis() {
        // An oversized, near-empty buffer shrinks back to the target, but only
        // once the quiet streak reaches the hysteresis threshold.
        let mut buf = BytesMut::with_capacity(OVERSIZED);
        let grown = buf.capacity();
        assert!(grown > TARGET_BUFFER_CAPACITY * BUFFER_SHRINK_FACTOR);
        let mut streak = 0;

        for _ in 0..BUFFER_SHRINK_HYSTERESIS - 1 {
            maybe_shrink_buffer(&mut buf, &mut streak);
            assert_eq!(buf.capacity(), grown, "must not shrink before hysteresis");
        }
        maybe_shrink_buffer(&mut buf, &mut streak);
        assert_eq!(buf.capacity(), TARGET_BUFFER_CAPACITY);
        assert_eq!(streak, 0);
    }

    #[test]
    fn a_busy_oversized_buffer_is_not_shrunk_and_resets_the_streak() {
        // While the residue still exceeds the target the buffer is legitimately
        // busy; shrinking is skipped and any accrued streak resets.
        let mut buf = BytesMut::with_capacity(OVERSIZED);
        let grown = buf.capacity();
        let mut streak = 0;

        // Build a partial streak on an empty buffer.
        for _ in 0..BUFFER_SHRINK_HYSTERESIS - 1 {
            maybe_shrink_buffer(&mut buf, &mut streak);
        }
        assert_eq!(streak, BUFFER_SHRINK_HYSTERESIS - 1);

        // A large residue arrives: streak resets, no shrink.
        buf.resize(TARGET_BUFFER_CAPACITY + 1, 0);
        maybe_shrink_buffer(&mut buf, &mut streak);
        assert_eq!(streak, 0);
        assert_eq!(buf.capacity(), grown);
    }
}
