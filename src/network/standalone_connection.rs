use crate::{
    ConnectionState, Error, Future, Result, RetryReason, TcpStreamReader, TcpStreamWriter,
    client::{BufferConfig, Config, PreparedCommand},
    commands::{
        ClusterCommands, ConnectionCommands, HelloOptions, SentinelCommands, ServerCommands,
    },
    resp::{BufferDecoder, Command, CommandEncoder, RespResponse, StateSlot},
    tcp_connect,
};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::{TcpTlsStreamReader, TcpTlsStreamWriter, tcp_tls_connect};
use bytes::BytesMut;
use futures_util::{SinkExt, Stream, StreamExt, task::noop_waker_ref};
use serde::de::DeserializeOwned;
use std::{
    future::IntoFuture,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{Instrument, debug, info_span, trace, warn};

/// Whether a recorded `CLIENT REPLY` command is the one that makes the connection
/// answer again — the only mode of the three whose own reply arrives.
fn command_turns_replies_on(command: &Command) -> bool {
    command
        .get_arg(1)
        .is_none_or(|mode| mode.eq_ignore_ascii_case(b"ON"))
}

/// Replaces `buf` with a fresh `buffers.read_capacity` buffer once it has been
/// oversized and near-empty for long enough, returning its high-water-mark
/// memory to the allocator. `BytesMut` has no `shrink_to_fit`, so replacement
/// is the only lever.
///
/// `small_streak` is the caller-owned hysteresis counter for this buffer.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the streak is reset the moment it reaches `shrink_hysteresis`, so it \
              never grows past that setting."
)]
fn maybe_shrink_buffer(buf: &mut BytesMut, small_streak: &mut usize, buffers: &BufferConfig) {
    // Part 1: ignore buffers that have not grown well past the target.
    // A saturated product is above any real capacity, so the buffer is left alone
    // — the safe direction for a shrink heuristic.
    if buf.capacity() <= buffers.read_capacity.saturating_mul(buffers.shrink_factor) {
        *small_streak = 0;
        return;
    }
    // The residue must fit the fresh buffer for the copy below to stay within
    // the target; if it does not, the buffer is legitimately busy right now.
    if buf.len() > buffers.read_capacity {
        *small_streak = 0;
        return;
    }
    // Part 2: require a streak of quiet observations before reallocating.
    *small_streak += 1;
    if *small_streak < buffers.shrink_hysteresis {
        return;
    }
    *small_streak = 0;
    let mut replacement = BytesMut::with_capacity(buffers.read_capacity);
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
    pub(crate) async fn connect(host: &str, port: u16, config: &Config) -> Result<Self> {
        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        if let Some(tls_config) = &config.tls_config {
            let (reader, writer) =
                tcp_tls_connect(host, port, tls_config, config.connect_timeout).await?;
            let framed_read = FramedRead::with_capacity(
                reader,
                BufferDecoder::with_config(config.buffers, config.limits),
                config.buffers.read_capacity,
            );
            let framed_write = FramedWrite::new(writer, CommandEncoder);
            Ok(Streams::TcpTls(framed_read, framed_write))
        } else {
            Self::connect_non_secure(host, port, config).await
        }

        #[cfg(not(any(feature = "native-tls", feature = "rustls")))]
        Self::connect_non_secure(host, port, config).await
    }

    pub(crate) async fn connect_non_secure(host: &str, port: u16, config: &Config) -> Result<Self> {
        let (reader, writer) = tcp_connect(host, port, config).await?;
        let framed_read = FramedRead::with_capacity(
            reader,
            BufferDecoder::with_config(config.buffers, config.limits),
            config.buffers.read_capacity,
        );
        let framed_write = FramedWrite::new(writer, CommandEncoder);
        Ok(Streams::Tcp(framed_read, framed_write))
    }
}

pub(crate) struct StandaloneConnection {
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
    /// Opens the client's own connection: the state a caller attached to the
    /// previous socket is replayed onto this one.
    pub(crate) async fn connect(
        host: &str,
        port: u16,
        config: &Config,
        connection_state: &mut ConnectionState,
    ) -> Result<Self> {
        Self::connect_inner(host, port, config, Some(connection_state)).await
    }

    /// Opens a connection that is **not** the caller's — cluster shard discovery,
    /// a probe to a Sentinel, the test-only `CLIENT KILL` connection. It carries
    /// no caller state: replaying their database, name or tracking mode onto a
    /// node they never addressed would be wrong.
    pub(crate) async fn connect_control(host: &str, port: u16, config: &Config) -> Result<Self> {
        Self::connect_inner(host, port, config, None).await
    }

    async fn connect_inner(
        host: &str,
        port: u16,
        config: &Config,
        connection_state: Option<&mut ConnectionState>,
    ) -> Result<Self> {
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

        // The handshake commands are logged like any other, so they need the same
        // span the network loop will run under. It can only exist from here: the
        // tag identifying the connection is what the lines above just built.
        let span = info_span!("connection", tag = %connection.tag);
        connection
            .post_connect(connection_state)
            .instrument(span)
            .await?;

        Ok(connection)
    }

    /// Returns the oversized read/write buffers to the allocator once they have
    /// been quiet long enough. Disjoint field borrows let the streak counters and
    /// `streams` be mutated together.
    fn shrink_read_buffer(&mut self) {
        let streak = &mut self.read_buffer_small_streak;
        let buffers = &self.config.buffers;
        match &mut self.streams {
            Streams::Tcp(framed_read, _) => {
                maybe_shrink_buffer(framed_read.read_buffer_mut(), streak, buffers)
            }
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(framed_read, _) => {
                maybe_shrink_buffer(framed_read.read_buffer_mut(), streak, buffers)
            }
        }
    }

    fn shrink_write_buffer(&mut self) {
        let streak = &mut self.write_buffer_small_streak;
        let buffers = &self.config.buffers;
        match &mut self.streams {
            Streams::Tcp(_, framed_write) => {
                maybe_shrink_buffer(framed_write.write_buffer_mut(), streak, buffers)
            }
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => {
                maybe_shrink_buffer(framed_write.write_buffer_mut(), streak, buffers)
            }
        }
    }

    async fn write(&mut self, command: &Command) -> Result<()> {
        debug!("Sending command: {command}");
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

    pub(crate) async fn feed(
        &mut self,
        command: &Command,
        _retry_reasons: &[RetryReason],
    ) -> Result<()> {
        debug!("Sending command: {command}");

        #[cfg(test)]
        if command.try_decrement_kill_connection_on_write() {
            let client_id = self.client_id().await?;
            let mut config = self.config.clone();
            "killer".clone_into(&mut config.connection_name);
            let mut connection =
                StandaloneConnection::connect_control(&self.host, self.port, &config).await?;
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

    pub(crate) async fn flush(&mut self) -> Result<()> {
        trace!("Flushing...");
        let result = match &mut self.streams {
            Streams::Tcp(_, framed_write) => framed_write.flush().await,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            Streams::TcpTls(_, framed_write) => framed_write.flush().await,
        };
        // The write buffer is now drained; reclaim it if it grew oversized.
        self.shrink_write_buffer();
        result
    }

    #[cfg_attr(
        test,
        expect(
            clippy::arithmetic_side_effects,
            reason = "the fault-injection countdown is only decremented inside `> 0`. It \
                      is `cfg(test)` state: no shipped build reaches this."
        )
    )]
    pub(crate) async fn read(&mut self) -> Option<Result<RespResponse>> {
        // Test-only: simulate the connection being closed before any response
        // is delivered, once the armed countdown expires.
        #[cfg(test)]
        if self.kill_connection_on_read_countdown > 0 {
            self.kill_connection_on_read_countdown -= 1;
            if self.kill_connection_on_read_countdown == 0 {
                debug!("Simulating a closed socket on read");
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
            match &result {
                Ok(response) => debug!("Received response {response:?}"),
                Err(err) => debug!("Received response {err:?}"),
            }
            Some(result)
        } else {
            debug!("Socked is closed");
            None
        }
    }

    #[cfg_attr(
        test,
        expect(
            clippy::arithmetic_side_effects,
            reason = "the fault-injection countdown is only decremented inside `> 0`. It \
                      is `cfg(test)` state: no shipped build reaches this."
        )
    )]
    pub(crate) fn try_read(&mut self) -> Poll<Option<Result<RespResponse>>> {
        // Test-only: mirror `read`'s simulated close on the drain path.
        #[cfg(test)]
        if self.kill_connection_on_read_countdown > 0 {
            self.kill_connection_on_read_countdown -= 1;
            if self.kill_connection_on_read_countdown == 0 {
                debug!("(try_read) Simulating a closed socket on read");
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
                match &result {
                    Ok(response) => debug!("(try_read) Received result {response:?}"),
                    Err(err) => debug!("(try_read) Received result {err:?}"),
                }
                Poll::Ready(Some(result))
            }
            Poll::Ready(None) => {
                debug!("Socket is closed");
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending, // Nothing to read right now
        }
    }

    pub(crate) async fn reconnect(
        &mut self,
        connection_state: Option<&mut ConnectionState>,
    ) -> Result<()> {
        self.streams = Streams::connect(&self.host, self.port, &self.config).await?;
        // Fresh streams carry fresh buffers, so the shrink hysteresis restarts.
        self.read_buffer_small_streak = 0;
        self.write_buffer_small_streak = 0;
        self.post_connect(connection_state).await?;

        Ok(())
    }

    async fn post_connect(&mut self, connection_state: Option<&mut ConnectionState>) -> Result<()> {
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

        // select database, unless the caller has since selected another one: the
        // replay below would immediately overwrite this, so emitting it would cost
        // a round-trip on every reconnection to reach a database nobody is on.
        let runtime_select = connection_state
            .as_ref()
            .is_some_and(|state| state.holds(StateSlot::Select));
        if self.config.database != 0 && !runtime_select {
            self.select(self.config.database).await?;
        }

        if let Some(connection_state) = connection_state {
            self.restore_connection_state(connection_state).await;
        }

        Ok(())
    }

    /// Replays the connection-attached state the caller set at runtime, which the
    /// handshake above cannot know about: it only ever restores what the config
    /// describes.
    ///
    /// A rejected replay is logged and its slot **dropped**. The state it carries is
    /// state the caller asked for and the server has now refused; a command that
    /// fails once fails on every reconnection, and turning that into a dead client
    /// would make a single bad `AUTH` fatal for the rest of the client's life.
    async fn restore_connection_state(&mut self, connection_state: &mut ConnectionState) {
        if let Some(refused) = self.replay_state(&connection_state.commands(), true).await {
            connection_state.forget(refused);
        }
    }

    /// Brings this connection up to `snapshot`, for a node a cluster topology change
    /// brought into the topology after the client's state was set.
    ///
    /// The snapshot is read-only here — the handler owns the registry — so a refused
    /// slot is logged and left in place rather than dropped.
    pub(crate) async fn restore_from_snapshot(&mut self, snapshot: &ConnectionState) {
        self.replay_state(&snapshot.commands(), false).await;
    }

    /// The single replay loop behind both entry points above. Returns the first slot
    /// the server refused, if any.
    ///
    /// `with_reply_mode` is false for a cluster node: silencing it would strand the
    /// read below, and `CLIENT REPLY OFF` is refused on a cluster client anyway.
    async fn replay_state(
        &mut self,
        commands: &[(StateSlot, Command)],
        with_reply_mode: bool,
    ) -> Option<StateSlot> {
        for (slot, command) in commands {
            let slot = *slot;
            if slot == StateSlot::ReplyMode && !with_reply_mode {
                continue;
            }

            if let Err(e) = self.write(command).await {
                warn!("Cannot restore {slot:?}: {e}");
                return None;
            }

            // Silencing the connection is the one restore that is not answered,
            // which is why `ReplyMode` is replayed last: nothing follows it that
            // would wait for a reply that can no longer come.
            if slot == StateSlot::ReplyMode && !command_turns_replies_on(command) {
                continue;
            }

            match self.read().await {
                Some(Ok(_)) => (),
                Some(Err(e)) => {
                    warn!("Cannot restore {slot:?}: {e}");
                    return Some(slot);
                }
                None => {
                    warn!("Connection closed while restoring {slot:?}");
                    return None;
                }
            }
        }

        None
    }

    pub(crate) fn get_version(&self) -> &str {
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
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::maybe_shrink_buffer;
    use crate::client::BufferConfig;
    use bytes::BytesMut;

    const BUFFERS: BufferConfig = BufferConfig::DEFAULT;
    const TARGET_BUFFER_CAPACITY: usize = BUFFERS.read_capacity;
    const BUFFER_SHRINK_FACTOR: usize = BUFFERS.shrink_factor;
    const BUFFER_SHRINK_HYSTERESIS: usize = BUFFERS.shrink_hysteresis;
    const OVERSIZED: usize = TARGET_BUFFER_CAPACITY * BUFFER_SHRINK_FACTOR + 1;

    #[test]
    fn does_not_shrink_a_buffer_within_the_factor() {
        // A buffer that never grew past factor × target is left alone and never
        // accrues a streak.
        let mut buf = BytesMut::with_capacity(TARGET_BUFFER_CAPACITY);
        let mut streak = 0;
        for _ in 0..BUFFER_SHRINK_HYSTERESIS * 2 {
            maybe_shrink_buffer(&mut buf, &mut streak, &BUFFERS);
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
            maybe_shrink_buffer(&mut buf, &mut streak, &BUFFERS);
            assert_eq!(buf.capacity(), grown, "must not shrink before hysteresis");
        }
        maybe_shrink_buffer(&mut buf, &mut streak, &BUFFERS);
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
            maybe_shrink_buffer(&mut buf, &mut streak, &BUFFERS);
        }
        assert_eq!(streak, BUFFER_SHRINK_HYSTERESIS - 1);

        // A large residue arrives: streak resets, no shrink.
        buf.resize(TARGET_BUFFER_CAPACITY + 1, 0);
        maybe_shrink_buffer(&mut buf, &mut streak, &BUFFERS);
        assert_eq!(streak, 0);
        assert_eq!(buf.capacity(), grown);
    }

    #[test]
    fn shrinks_to_the_configured_target_after_the_configured_hysteresis() {
        // The policy must follow `BufferConfig`, not the historical constants:
        // a caller who lowers the target and shortens the streak sees the buffer
        // released sooner and to their own size.
        let buffers = BufferConfig {
            read_capacity: 4 * 1024,
            shrink_factor: 2,
            shrink_hysteresis: 3,
            ..BufferConfig::DEFAULT
        };
        let mut buf = BytesMut::with_capacity(buffers.read_capacity * buffers.shrink_factor + 1);
        let grown = buf.capacity();
        let mut streak = 0;

        for _ in 0..buffers.shrink_hysteresis - 1 {
            maybe_shrink_buffer(&mut buf, &mut streak, &buffers);
            assert_eq!(buf.capacity(), grown, "must not shrink before hysteresis");
        }
        maybe_shrink_buffer(&mut buf, &mut streak, &buffers);
        assert_eq!(buf.capacity(), buffers.read_capacity);
    }
}
