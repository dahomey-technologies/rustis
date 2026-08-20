use crate::{
    ClientError, Error, Result, TimeoutKind,
    client::{
        ClientStats, ClientTrackingInvalidationStream, CommandFuture, CommandInterceptor, Config,
        ExclusiveClient, IntoConfig, Message, MonitorStream, Pipeline, PreparedCommand, ProbeLabel,
        PubSubStream, ServerConfig, State, StatsRecorder, Transaction, bounded_channel,
        command_traits::*, record_probe,
    },
    commands::PubSubCommands,
    network::{
        JoinHandle, MsgSender, NetworkHandler, PubSubReceiver, PubSubSender, PushReceiver,
        PushSender, ReconnectReceiver, ReconnectSender, ResultReceiver, ResultSender,
        ResultsReceiver, ResultsSender, timeout, timeout_future,
    },
    resp::{Command, CommandArgs, CommandArgsMut, RespResponse, SubscriptionType, cmd},
};
use bytes::Bytes;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    future::IntoFuture,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{info, trace};

/// Client with a unique connection to a Redis server.
/// State shared by every clone of a [`Client`] over a single connection.
///
/// The message sender and the network task join handle live behind **one**
/// `Arc`, so a single reference count governs both. Shutting the connection
/// down is then gated on [`Arc::into_inner`], which hands the last owner
/// exclusive access and — crucially — returns `Some` to exactly one caller even
/// when several clones drop (or `close`) concurrently. Two independent `Arc`s
/// decided with `try_unwrap` allowed two threads to each observe strong-count 2
/// and both back off, leaking the task, socket and buffers forever.
struct ClientShared {
    msg_sender: MsgSender,
    network_task_join_handle: JoinHandle<()>,
}

/// What [`Client::close`] did.
///
/// A connection is shared by every clone of a client, so a `close` call that
/// finds a clone still alive shuts nothing down. The two cases are told apart
/// here rather than both reported as `Ok(())`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    /// This was the last handle: the send channel is closed and the network task
    /// has finished.
    Closed,
    /// A clone is still holding the connection, which stays up. Nothing was sent
    /// and nothing was shut down.
    StillShared,
}

#[derive(Clone)]
pub struct Client {
    /// `Option` only so a dropping/closing clone can swap its reference out of
    /// `&mut self` before calling [`Arc::into_inner`]; a live client always
    /// holds `Some`.
    shared: Arc<Option<ClientShared>>,
    reconnect_sender: ReconnectSender,
    command_timeout: Duration,
    retry_on_error: bool,
    connection_tag: Arc<str>,
    /// Whether this client talks to a Redis Cluster, which constrains what a
    /// transaction may contain.
    is_cluster: bool,
    /// Memory budget handed to each pub/sub stream this client opens, from
    /// `Config::backpressure.max_pubsub_bytes`.
    max_pubsub_bytes: usize,
    /// Memory budget handed to each push sink this client opens, from
    /// `Config::backpressure.max_push_bytes`.
    max_push_bytes: usize,
    /// The configuration this client was built from, kept whole so a caller can
    /// read back what it connected with. The scalars above are copies on the
    /// hot path, not a second source of truth.
    config: Arc<Config>,
    /// Counters the network task publishes as it runs.
    stats: Arc<StatsRecorder>,
    /// Copied out of the config so the send path reads one `Option` instead of
    /// walking into the whole `Config` on every command.
    interceptor: Option<Arc<dyn CommandInterceptor>>,
}

impl Client {
    /// Connects asynchronously to the Redis server.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    #[inline]
    pub async fn connect(config: impl IntoConfig) -> Result<Self> {
        let config = config.into_config()?;
        let command_timeout = config.command_timeout;
        let retry_on_error = config.retry_on_error;
        let is_cluster = matches!(config.server, ServerConfig::Cluster(_));
        let max_pubsub_bytes = config.backpressure.max_pubsub_bytes;
        let max_push_bytes = config.backpressure.max_push_bytes;
        let interceptor = config
            .interceptor
            .as_ref()
            .map(|interceptor| Arc::clone(interceptor.get()));
        let (msg_sender, network_task_join_handle, reconnect_sender, connection_tag, stats) =
            NetworkHandler::connect(config.clone()).await?;

        Ok(Self {
            shared: Arc::new(Some(ClientShared {
                msg_sender,
                network_task_join_handle,
            })),
            reconnect_sender,
            command_timeout,
            retry_on_error,
            connection_tag,
            is_cluster,
            max_pubsub_bytes,
            max_push_bytes,
            config: Arc::new(config),
            stats,
            interceptor,
        })
    }

    /// The configuration this client was built from.
    ///
    /// A pool wrapper, a health checker or a telemetry exporter can then ask a
    /// client what it was configured with instead of being handed the `Config`
    /// separately. It is the value after parsing, so a client built from a URI
    /// reads back every default the URI left unsaid.
    ///
    /// # Example
    /// ```
    /// use rustis::{client::{Client, ServerConfig}, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = Client::connect("127.0.0.1:6379").await?;
    /// assert!(matches!(client.config().server, ServerConfig::Standalone { .. }));
    /// # Ok(())
    /// # }
    /// ```
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// What the connection is doing right now: queue depth, shed commands,
    /// reconnections.
    ///
    /// This is the other half of [`BackpressureConfig`](crate::client::BackpressureConfig):
    /// the budget is sized against
    /// [`queued_bytes_high_water`](ClientStats::queued_bytes_high_water), and
    /// whether it is being hit is [`shed_commands`](ClientStats::shed_commands).
    /// Every clone of a client reads the same counters, one connection having
    /// one queue.
    ///
    /// # Example
    /// ```
    /// use rustis::{client::Client, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = Client::connect("127.0.0.1:6379").await?;
    /// assert_eq!(0, client.stats().shed_commands);
    /// # Ok(())
    /// # }
    /// ```
    pub fn stats(&self) -> ClientStats {
        self.stats.snapshot()
    }

    /// Whether the link to the server is up.
    ///
    /// `false` covers a link that is down and one that is backing off between
    /// attempts, both of which recover on their own. The state that does not is
    /// [`is_terminated`](Self::is_terminated).
    pub fn is_connected(&self) -> bool {
        self.stats.connected()
    }

    /// The server version the handshake reported, refreshed at every
    /// reconnection.
    ///
    /// `None` for a cluster: its nodes have versions of their own, so one string
    /// would have to pick a node and hide the rest. Reading this replaces
    /// re-issuing `HELLO` to branch on a version-dependent behaviour.
    pub fn server_version(&self) -> Option<Arc<str>> {
        self.stats.server_version()
    }

    /// Whether this client is connected to a Redis Cluster.
    pub(crate) fn is_cluster(&self) -> bool {
        self.is_cluster
    }

    #[allow(dead_code)]
    pub(crate) fn connection_tag(&self) -> &str {
        &self.connection_tag
    }

    /// Whether this client is finished for good.
    ///
    /// The network task behind the client ends when the connection is gone
    /// beyond recovery: a non-zero
    /// [`ReconnectionConfig`](crate::client::ReconnectionConfig) budget
    /// exhausted, or the last handle dropped. Every command issued afterwards
    /// fails, including long after the server has come back, and the only
    /// recovery is a new client from [`Client::connect`](Self::connect).
    ///
    /// This is what a liveness probe reads: the state is otherwise invisible, a
    /// process staying alive and serving traffic it cannot answer. Keep the
    /// default budget of `0` in a long-lived service and it never happens.
    ///
    /// It reports the task, not the link. `false` says nothing about a connection
    /// that is merely idle, disconnected, or backing off between attempts — those
    /// all recover on their own.
    ///
    /// # Example
    /// ```
    /// use rustis::{client::Client, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = Client::connect("127.0.0.1:6379").await?;
    /// assert!(!client.is_terminated());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_terminated(&self) -> bool {
        self.shared
            .as_ref()
            .as_ref()
            .is_some_and(|shared| shared.network_task_join_handle.is_finished())
    }

    /// Ends the connection, if this handle is the last one on it.
    ///
    /// A [`Client`] is a handle: several clones share one connection, one queue
    /// and one network task. Only the last handle can end them, so the outcome
    /// is what the call reports:
    /// [`CloseOutcome::Closed`](crate::client::CloseOutcome::Closed) means the
    /// send channel was closed and the network task has finished, and
    /// [`StillShared`](crate::client::CloseOutcome::StillShared) means a clone
    /// is still using the connection and nothing was shut down. `Ok` alone is
    /// not the answer, which is why the outcome is returned rather than
    /// discarded: a shutdown path that treats `Ok(())` as "drained" would be
    /// wrong for every clone but one.
    ///
    /// Awaiting a `Closed` outcome awaits the network task, so the socket and
    /// the buffers are gone when it returns. Dropping the last handle does the
    /// same shutdown, without waiting for it.
    ///
    /// # Example
    /// ```
    /// use rustis::{client::{Client, CloseOutcome}, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = Client::connect("127.0.0.1:6379").await?;
    /// let clone = client.clone();
    ///
    /// assert_eq!(CloseOutcome::StillShared, client.close().await?);
    /// assert_eq!(CloseOutcome::Closed, clone.close().await?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn close(mut self) -> Result<CloseOutcome> {
        let mut shared: Arc<Option<ClientShared>> = Arc::new(None);
        std::mem::swap(&mut shared, &mut self.shared);

        // stop the network loop if we are the last owner of the shared state;
        // `into_inner` makes that determination race-free against a concurrent
        // `close`/`Drop` (see `ClientShared`).
        if let Some(Some(shared)) = Arc::into_inner(shared) {
            let ClientShared {
                msg_sender,
                network_task_join_handle,
            } = shared;
            // Dropping the last strong sender closes the channel, which is what
            // ends the network loop; the task keeps a weak handle only, so this
            // must happen before awaiting it, otherwise the loop never sees the
            // channel close and the await deadlocks.
            drop(msg_sender);
            network_task_join_handle.await?;
            return Ok(CloseOutcome::Closed);
        }

        Ok(CloseOutcome::StillShared)
    }

    /// Turns this handle into an [`ExclusiveClient`], the client that owns its
    /// connection and may therefore run
    /// [`BlockingCommands`](crate::commands::BlockingCommands) and
    /// [`watch`](crate::commands::TransactionCommands::watch).
    ///
    /// The conversion succeeds only when this is the **sole** handle on the
    /// connection. A surviving clone would keep sending commands over a
    /// connection the exclusive client believes is its own, which is the very
    /// situation the two client types exist to prevent — so the check is what
    /// gives [`ExclusiveClient`] its meaning, not a formality. Streams already
    /// opened from this client ([`create_pub_sub`](Self::create_pub_sub), a
    /// [`Transaction`], a [`MonitorStream`]) hold a handle too and count here.
    ///
    /// The client is consumed either way: on failure other handles exist by
    /// definition, so nothing is lost with it. Two clones converting
    /// concurrently both observe the other and both fail, which is the safe
    /// direction.
    ///
    /// # Errors
    /// [`ClientError::NotExclusive`] when another handle on the same connection
    /// is alive.
    ///
    /// # Example
    /// ```
    /// use rustis::{client::Client, commands::BlockingCommands, Result};
    ///
    /// # async fn example() -> Result<()> {
    /// let client = Client::connect("127.0.0.1:6379").await?.into_exclusive()?;
    /// let result: Option<(String, String)> = client.blpop("key", 30.).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn into_exclusive(self) -> Result<ExclusiveClient> {
        if Arc::strong_count(&self.shared) != 1 {
            return Err(Error::from(ClientError::NotExclusive));
        }

        Ok(ExclusiveClient::from_client(self))
    }

    /// Used to receive notifications when the client reconnects to the Redis server.
    ///
    /// To turn this receiver into a Stream, you can use the
    /// [`BroadcastStream`](https://docs.rs/tokio-stream/latest/tokio_stream/wrappers/struct.BroadcastStream.html) wrapper.
    pub fn on_reconnect(&self) -> ReconnectReceiver {
        self.reconnect_sender.subscribe()
    }

    /// Send an arbitrary command to the server.
    ///
    /// This is used primarily intended for implementing high level commands API
    /// but may also be used to provide access to new features that lack a direct API.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    /// * `retry_on_error` - retry to send the command on network error.
    ///   * `None` - default behaviour defined in [`Config::retry_on_error`](crate::client::Config::retry_on_error)
    ///   * `Some(true)` - retry sending command on network error
    ///   * `Some(false)` - do not retry sending command on network error
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    ///
    /// # Warning
    /// In Cluster mode, the arguments that are Redis keys must be added with
    /// [`CommandBuilder::key`](crate::resp::CommandBuilder::key): a command built with
    /// [`arg`](crate::resp::CommandBuilder::arg) alone carries no slot and is sent to a
    /// **random node**. A multi-key command such as `MSET` also requires all its keys to hash
    /// to the same slot, which the `{my}` hash tag guarantees in the example below.
    ///
    /// Unless `R` is an [`Option`], a `nil` reply decodes as the neutral value of `R`
    /// (`0`, `0.0`, `false`, `""`) instead of being rejected. Declare `Option<R>` when the
    /// command can reply `nil`. See [Command results](crate::resp#command-results).
    ///
    /// Dropping the returned future does not cancel the command: the message is already queued,
    /// so it is sent and executed by the server and only the reply is discarded. A `timeout`, a
    /// `select!` or an aborted task therefore leaves a non-idempotent command applied. See
    /// [Cancellation and timeouts](crate::client#cancellation-and-timeouts).
    ///
    /// # Example
    /// ```
    /// use rustis::{client::Client, commands::{FlushingMode, ServerCommands}, resp::cmd, Result};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = Client::connect("127.0.0.1:6379").await?;
    ///
    ///     client.flushall(FlushingMode::Sync).await?;
    ///
    ///     client
    ///         .send::<()>(
    ///             cmd("MSET")
    ///                 .key("{my}key1")
    ///                 .arg("value1")
    ///                 .key("{my}key2")
    ///                 .arg("value2")
    ///                 .key("{my}key3")
    ///                 .arg("value3")
    ///                 .key("{my}key4")
    ///                 .arg("value4"),
    ///             None,
    ///         )
    ///         .await?;
    ///
    ///     let values: Vec<String> = client
    ///         .send(
    ///             cmd("MGET")
    ///                 .key("{my}key1")
    ///                 .key("{my}key2")
    ///                 .key("{my}key3")
    ///                 .key("{my}key4"),
    ///             None,
    ///         )
    ///         .await?;
    ///
    ///     assert_eq!(vec!["value1".to_owned(), "value2".to_owned(), "value3".to_owned(), "value4".to_owned()], values);
    ///
    ///     Ok(())
    /// }
    /// ```
    #[inline]
    pub async fn send<T: DeserializeOwned>(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<T> {
        // The one place holding both the command and the type the caller
        // declared for its reply, so it is where the two can be confronted with
        // what the server actually answers.
        #[cfg(test)]
        let command = command.into();
        #[cfg(test)]
        let probe_label = crate::tests::response_probe::label(&command);

        #[cfg(not(test))]
        let probe_label = ();

        // Concluded here rather than at the reply: a server error and a decode
        // mismatch are both born below, and an interceptor that missed them
        // would report as successful a command its caller sees fail.
        let started_at = self.started_at();
        let result = async {
            let (response, command_name) = self.internal_send(command, retry_on_error).await?;
            Self::finish_send(&response, command_name.clone(), probe_label)
                .map(|r| (r, command_name))
        }
        .await;
        self.notify_completion(
            result.as_ref().ok().and_then(|(_, name)| name.as_ref()),
            started_at,
            result.as_ref().err(),
        );
        result.map(|(response, _)| response)
    }

    /// Turns a reply into the type the caller declared for it, and names the
    /// command in whatever error that produces.
    ///
    /// Shared by [`send`](Self::send) and [`CommandFuture`], so the ergonomic
    /// and the generic path decode a reply the same way rather than each
    /// keeping its own copy of these three lines.
    #[inline]
    pub(crate) fn finish_send<T: DeserializeOwned>(
        response: &RespResponse,
        command_name: Option<Bytes>,
        probe_label: ProbeLabel,
    ) -> Result<T> {
        let result = response.to();

        // The outcome is recorded alongside the shape: a mismatch the decoder
        // refuses is a mismatch the caller was told about, where a mismatch it
        // coerces is the silent one this probe exists for.
        record_probe::<T>(probe_label, response, result.is_ok());

        Self::name_command(result, command_name)
    }

    /// Hands a command to the network task and returns what a [`CommandFuture`]
    /// then waits on, alongside the command name and the probe label it needs
    /// to finish. Called from the future's first poll, so that building one and
    /// dropping it sends nothing.
    #[inline]
    pub(crate) fn start_send<'a>(
        &self,
        command: Command,
        retry_on_error: Option<bool>,
    ) -> (State<'a>, Option<Bytes>, ProbeLabel) {
        #[cfg(test)]
        let probe_label = crate::tests::response_probe::label(&command);
        #[cfg(not(test))]
        let probe_label = ();

        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();
        let message = Message::single(
            command,
            result_sender,
            retry_on_error.unwrap_or(self.retry_on_error),
        );

        let command_name = match self.send_message(message) {
            Ok(command_name) => command_name,
            Err(error) => {
                return (State::Failed { error: Some(error) }, None, probe_label);
            }
        };

        let state = if self.command_timeout != Duration::ZERO {
            State::Timed {
                receiver: timeout_future(self.command_timeout, result_receiver),
            }
        } else {
            State::Waiting {
                receiver: result_receiver,
            }
        };

        (state, command_name, probe_label)
    }

    /// Names `command_name` in the error of `result`, when there is one.
    #[inline]
    pub(crate) fn name_command<T>(result: Result<T>, command_name: Option<Bytes>) -> Result<T> {
        match (result, command_name) {
            (Err(e), Some(command)) => Err(e.with_command(command)),
            (result, _) => result,
        }
    }

    /// Sends one command and awaits its reply, returning the name of the command
    /// alongside it.
    ///
    /// The name is handed back because the reply is not the end of the road: the
    /// caller still deserializes it, and both a server error and a type mismatch
    /// are born there, past the point where the network task could have named
    /// them.
    #[inline]
    pub(crate) async fn internal_send(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<(RespResponse, Option<Bytes>)> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();
        let message = Message::single(
            command.into(),
            result_sender,
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        let command_name = self.send_message(message)?;

        let response = self
            .await_result(result_receiver, command_name.clone())
            .await?;
        Ok((response, command_name))
    }

    /// Reads the clock only when an interceptor is installed: the two readings
    /// per command would otherwise be a cost on the hot path for nobody.
    #[inline]
    fn started_at(&self) -> Option<Instant> {
        self.interceptor.as_ref().map(|_| Instant::now())
    }

    /// The interceptor installed on this client, if any.
    #[inline]
    pub(crate) fn interceptor(&self) -> Option<&Arc<dyn CommandInterceptor>> {
        self.interceptor.as_ref()
    }

    /// Tells the interceptor a command resolved, if there is one.
    #[inline]
    fn notify_completion(
        &self,
        command_name: Option<&Bytes>,
        started_at: Option<Instant>,
        error: Option<&Error>,
    ) {
        if let Some(interceptor) = &self.interceptor
            && let Some(started_at) = started_at
        {
            interceptor.on_complete(
                command_name.map_or(&[][..], |name| name.as_ref()),
                started_at.elapsed(),
                error,
            );
        }
    }

    /// Await a single-response oneshot, applying the configured `command_timeout`
    /// so subscribe/monitor callers honour the same contract as regular sends.
    #[inline]
    async fn await_result(
        &self,
        result_receiver: ResultReceiver,
        command_name: Option<Bytes>,
    ) -> Result<RespResponse> {
        if self.command_timeout != Duration::ZERO {
            Self::name_command(
                timeout(self.command_timeout, TimeoutKind::Command, result_receiver).await,
                command_name,
            )??
        } else {
            result_receiver.await?
        }
    }

    /// Send command to the Redis server and forget its response.
    ///
    /// # Arguments
    /// * `command` - generic [`Command`](crate::resp::Command) meant to be sent to the Redis server.
    /// * `retry_on_error` - retry to send the command on network error.
    ///   * `None` - default behaviour defined in [`Config::retry_on_error`](crate::client::Config::retry_on_error)
    ///   * `Some(true)` - retry sending command on network error
    ///   * `Some(false)` - do not retry sending command on network error
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    #[inline]
    pub fn send_and_forget(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<()> {
        let message = Message::single_forget(
            command.into(),
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        self.send_message(message)?;
        Ok(())
    }

    /// Send a batch of commands to the Redis server.
    ///
    /// # Arguments
    /// * `commands` - batch of generic [`Command`](crate::resp::Command)s meant to be sent to the Redis server.
    /// * `retry_on_error` - retry to send the command batch on network error.
    ///   * `None` - default behaviour defined in [`Config::retry_on_error`](crate::client::Config::retry_on_error)
    ///   * `Some(true)` - retry sending batch on network error
    ///   * `Some(false)` - do not retry sending batch on network error
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the send operation
    ///
    /// Each reply is paired with the name of the command that drew it. A batch
    /// reply is deserialized per command, so an error born there belongs to one
    /// command of the batch and not to the batch as a whole: naming it after the
    /// first command would point at the wrong one.
    #[inline]
    pub(crate) async fn internal_send_batch(
        &self,
        commands: Vec<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<Vec<(RespResponse, Bytes)>> {
        let (results_sender, results_receiver): (ResultsSender, ResultsReceiver) =
            tokio::sync::oneshot::channel();
        let command_names: Vec<Bytes> = commands.iter().map(Command::name_bytes).collect();
        let message = Message::batch(
            commands,
            results_sender,
            retry_on_error.unwrap_or(self.retry_on_error),
        );
        let started_at = self.started_at();
        let command_name = match self.send_message(message) {
            Ok(command_name) => command_name,
            Err(e) => {
                self.notify_completion(None, started_at, Some(&e));
                return Err(e);
            }
        };

        let results = self
            .await_batch_results(results_receiver, command_name)
            .await;
        // A batch resolves as one unit, so it is announced under no command name
        // rather than under the first of the commands it carries.
        self.notify_completion(None, started_at, results.as_ref().err());

        Ok(results?.into_iter().zip(command_names).collect())
    }

    /// Awaits a batch's replies under the configured `command_timeout`.
    #[inline]
    async fn await_batch_results(
        &self,
        results_receiver: ResultsReceiver,
        command_name: Option<Bytes>,
    ) -> Result<Vec<RespResponse>> {
        if self.command_timeout != Duration::ZERO {
            Self::name_command(
                timeout(self.command_timeout, TimeoutKind::Command, results_receiver).await,
                command_name,
            )??
        } else {
            results_receiver.await?
        }
    }

    #[inline]
    /// Hands `message` to the network task, and returns the name of the command
    /// it carries so the caller can name it in a timeout — the one failure the
    /// network task never sees, and therefore never names itself.
    fn send_message(&self, mut message: Message) -> Result<Option<Bytes>> {
        // Surface any serialization error deferred by the fluent builder (a
        // failing user `Serialize` impl) before the command reaches the network
        // layer, so the caller gets a clean error instead of a panic.
        for command in message.commands_mut() {
            if let Some(error) = command.take_serialization_error() {
                return Err(error.with_command(command.name_bytes()));
            }
        }

        // Every command the client sends passes here — single, batched, and the
        // subscribe/monitor commands the client issues on its own behalf — so
        // this is the one place an interceptor has to be consulted. It runs
        // before the slots are computed, so a command it rewrites is routed on
        // what it left rather than on what it was handed.
        if let Some(interceptor) = &self.interceptor {
            for command in message.commands_mut() {
                interceptor.on_command(command);
            }
        }

        // Compute cluster hash slots here, on the caller thread, and only in
        // cluster mode. This keeps CRC16 off the shared network thread (the
        // multiplexer domain) while sparing standalone clients the cost.
        if self.is_cluster {
            for command in message.commands_mut() {
                command.compute_slots();
            }
        }

        // Both failures below deny a specific command, so they name it: the
        // message never reaches the network task, which is what would otherwise
        // have attached it.
        let command_name = message.command_name();
        if let Some(shared) = self.shared.as_ref() {
            trace!(
                tag = %self.connection_tag,
                "Will enqueue message: {message:?}"
            );
            match shared.msg_sender.send(message) {
                Ok(()) => Ok(command_name),
                Err(e) => {
                    info!("{e}");
                    Self::name_command(
                        Err(Error::from(ClientError::DisconnectedFromServer)),
                        command_name,
                    )
                }
            }
        } else {
            Self::name_command(Err(Error::from(ClientError::InvalidChannel)), command_name)
        }
    }

    /// Create a new transaction
    #[inline]
    pub fn create_transaction(&self) -> Transaction {
        Transaction::new(self.clone())
    }

    /// Create a new pipeline
    #[inline]
    pub fn create_pipeline<'a>(&'a self) -> Pipeline<'a> {
        Pipeline::new(self)
    }

    /// Create a new pub sub stream with no upfront subscription
    #[inline]
    pub fn create_pub_sub(&self) -> PubSubStream {
        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);
        PubSubStream::new(pub_sub_sender, pub_sub_receiver, self.clone())
    }

    /// Create a stream of client-side caching invalidations.
    ///
    /// The stream yields the keys Redis has invalidated, as
    /// [`BulkString`](crate::resp::BulkString) — Redis keys are binary-safe.
    /// Enable tracking on the same client with
    /// [`client_tracking`](crate::commands::ConnectionCommands::client_tracking)
    /// for the server to start sending them.
    ///
    /// ```
    /// use rustis::{client::{Client, ClientTrackingInvalidationStream}, Result};
    ///
    /// async fn watch(client: &Client) -> Result<ClientTrackingInvalidationStream> {
    ///     client.create_client_tracking_invalidation_stream()
    /// }
    /// ```
    pub fn create_client_tracking_invalidation_stream(
        &self,
    ) -> Result<ClientTrackingInvalidationStream> {
        let (push_sender, push_receiver): (PushSender, PushReceiver) =
            bounded_channel(self.max_push_bytes);
        let message = Message::client_tracking_invalidation(push_sender);
        self.send_message(message)?;
        Ok(ClientTrackingInvalidationStream::new(push_receiver))
    }

    /// Puts the connection in monitoring mode and streams what the server
    /// echoes back.
    ///
    /// `MONITOR` never returns, so it holds the connection for good: only
    /// [`ExclusiveClient`] exposes it, through
    /// [`BlockingCommands::monitor`](crate::commands::BlockingCommands::monitor).
    pub(crate) async fn monitor_stream(&self) -> Result<MonitorStream> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();
        let (push_sender, push_receiver): (PushSender, PushReceiver) =
            bounded_channel(self.max_push_bytes);

        let message = Message::monitor(cmd("MONITOR").into(), result_sender, push_sender);

        let command_name = self.send_message(message)?;

        let _bytes = self.await_result(result_receiver, command_name).await?;
        Ok(MonitorStream::new(push_receiver, self.clone()))
    }

    pub(crate) async fn subscribe_from_pub_sub_sender(
        &self,
        channels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();

        let pub_sub_senders = channels
            .into_iter()
            .map(|c| (c, pub_sub_sender.clone()))
            .collect();

        let message = Message::pub_sub(
            cmd("SUBSCRIBE").arg(channels).into(),
            result_sender,
            SubscriptionType::Channel,
            pub_sub_senders,
        );

        let command_name = self.send_message(message)?;

        self.await_result(result_receiver, command_name)
            .await?
            .to::<()>()
    }

    pub(crate) async fn psubscribe_from_pub_sub_sender(
        &self,
        patterns: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();

        let pub_sub_senders = patterns
            .into_iter()
            .map(|c| (c, pub_sub_sender.clone()))
            .collect();

        let message = Message::pub_sub(
            cmd("PSUBSCRIBE").arg(patterns).into(),
            result_sender,
            SubscriptionType::Pattern,
            pub_sub_senders,
        );

        let command_name = self.send_message(message)?;

        self.await_result(result_receiver, command_name)
            .await?
            .to::<()>()
    }

    pub(crate) async fn ssubscribe_from_pub_sub_sender(
        &self,
        shardchannels: &CommandArgs,
        pub_sub_sender: &PubSubSender,
    ) -> Result<()> {
        let (result_sender, result_receiver): (ResultSender, ResultReceiver) =
            tokio::sync::oneshot::channel();

        let pub_sub_senders = shardchannels
            .into_iter()
            .map(|c| (c, pub_sub_sender.clone()))
            .collect();

        let message = Message::pub_sub(
            cmd("SSUBSCRIBE").keys(shardchannels).into(),
            result_sender,
            SubscriptionType::ShardChannel,
            pub_sub_senders,
        );

        let command_name = self.send_message(message)?;

        self.await_result(result_receiver, command_name)
            .await?
            .to::<()>()
    }
}

/// Extension trait dedicated to [`PreparedCommand`](crate::client::PreparedCommand)
/// to add specific methods for the [`Client`](crate::client::Client) executor
pub trait ClientPreparedCommand<'a, R> {
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()>;
}

impl<'a, R: DeserializeOwned> ClientPreparedCommand<'a, R> for PreparedCommand<'a, &'a Client, R> {
    /// Send command and forget its response
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occur during the send operation
    fn forget(self) -> Result<()> {
        self.executor
            .send_and_forget(self.command, self.retry_on_error)
    }
}

impl<'a, R: DeserializeOwned + 'a> IntoFuture for PreparedCommand<'a, &'a Client, R> {
    type Output = Result<R>;
    type IntoFuture = CommandFuture<'a, R>;

    #[inline]
    fn into_future(self) -> Self::IntoFuture {
        CommandFuture::new(self.executor, self.command, self.retry_on_error)
    }
}

impl_shared_command_traits!(Client);

impl<'a> PubSubCommands<'a> for &'a Client {
    #[inline]
    async fn subscribe(self, channels: impl Serialize) -> Result<PubSubStream> {
        let channels = CommandArgsMut::default().arg(channels).freeze();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);

        self.subscribe_from_pub_sub_sender(&channels, &pub_sub_sender)
            .await?;

        Ok(PubSubStream::from_channels(
            channels,
            pub_sub_sender,
            pub_sub_receiver,
            self.clone(),
        ))
    }

    #[inline]
    async fn psubscribe(self, patterns: impl Serialize) -> Result<PubSubStream> {
        let patterns = CommandArgsMut::default().arg(patterns).freeze();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);

        self.psubscribe_from_pub_sub_sender(&patterns, &pub_sub_sender)
            .await?;

        Ok(PubSubStream::from_patterns(
            patterns,
            pub_sub_sender,
            pub_sub_receiver,
            self.clone(),
        ))
    }

    #[inline]
    async fn ssubscribe(self, shardchannels: impl Serialize) -> Result<PubSubStream> {
        let shardchannels = CommandArgsMut::default().arg(shardchannels).freeze();

        let (pub_sub_sender, pub_sub_receiver): (PubSubSender, PubSubReceiver) =
            bounded_channel(self.max_pubsub_bytes);

        self.ssubscribe_from_pub_sub_sender(&shardchannels, &pub_sub_sender)
            .await?;

        Ok(PubSubStream::from_shardchannels(
            shardchannels,
            pub_sub_sender,
            pub_sub_receiver,
            self.clone(),
        ))
    }
}
