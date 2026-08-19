use crate::{
    Result,
    client::{
        Client, ClientPreparedCommand, ClientTrackingInvalidationStream, CommandFuture, IntoConfig,
        Pipeline, PreparedCommand, PubSubStream, Transaction, command_traits::*,
    },
    commands::{BlockingCommands, PubSubCommands, TransactionCommands},
    network::ReconnectReceiver,
    resp::{Command, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use std::future::IntoFuture;

#[cfg(doc)]
use crate::client::MonitorStream;

/// Client that owns its connection, and may therefore block it.
///
/// A [`Client`] is clonable, and every clone multiplexes its commands over one
/// shared connection. Two families of commands are incompatible with that
/// sharing: [`BlockingCommands`], which hold the connection until they return,
/// and [`watch`](TransactionCommands::watch), whose watched state belongs to the
/// connection rather than to the handle that asked for it. On a multiplexed
/// client they stall or corrupt the work of every other clone.
///
/// `ExclusiveClient` is that guarantee expressed in the type system: it is
/// **not** [`Clone`], and it is the only client the two families are
/// implemented for. It carries every other command family as well, so nothing
/// is given up by choosing it.
///
/// # Getting one
///
/// [`connect`](Self::connect) opens a connection of its own. Alternatively,
/// [`Client::into_exclusive`] converts an existing client, and refuses when
/// another handle on the connection is alive.
///
/// ```
/// use rustis::{client::ExclusiveClient, commands::{BlockingCommands, TransactionCommands}, Result};
///
/// # async fn example() -> Result<()> {
/// let client = ExclusiveClient::connect("127.0.0.1:6379").await?;
///
/// let result: Option<(String, String)> = client.blpop("key", 30.).await?;
/// client.watch("key").await?;
/// # Ok(())
/// # }
/// ```
///
/// A blocking command does not compile on a [`Client`]:
///
/// ```compile_fail
/// use rustis::{client::Client, commands::BlockingCommands, Result};
///
/// # async fn example() -> Result<()> {
/// let client = Client::connect("127.0.0.1:6379").await?;
/// let result: Option<(String, String)> = client.blpop("key", 30.).await?;
/// # Ok(())
/// # }
/// ```
///
/// and an exclusive client cannot be cloned into a second handle:
///
/// ```compile_fail
/// use rustis::{client::ExclusiveClient, Result};
///
/// # async fn example() -> Result<()> {
/// let client = ExclusiveClient::connect("127.0.0.1:6379").await?;
/// let second = client.clone();
/// # Ok(())
/// # }
/// ```
pub struct ExclusiveClient {
    inner: Client,
}

impl ExclusiveClient {
    /// Connects asynchronously to the Redis server, on a connection this client
    /// alone will use.
    ///
    /// # Errors
    /// Any Redis driver [`Error`](crate::Error) that occurs during the connection operation
    #[inline]
    pub async fn connect(config: impl IntoConfig) -> Result<Self> {
        Ok(Self::from_client(Client::connect(config).await?))
    }

    /// Wraps a client whose exclusivity has already been established.
    pub(crate) fn from_client(inner: Client) -> Self {
        Self { inner }
    }

    /// The multiplexed handle underneath, for the delegating impls below and for
    /// the pool. Deliberately not public: handing out a `&Client` would let a
    /// caller clone a second handle onto a connection this type promises is
    /// exclusive.
    pub(crate) fn inner(&self) -> &Client {
        &self.inner
    }

    /// Gives up exclusivity and returns a clonable [`Client`] on the same
    /// connection.
    ///
    /// The blocking commands and [`watch`](TransactionCommands::watch) are gone
    /// from the returned handle, which is the point: after this call the
    /// connection may be shared.
    #[inline]
    pub fn into_multiplexed(self) -> Client {
        self.inner
    }

    /// See [`Client::close`].
    #[inline]
    pub async fn close(self) -> Result<()> {
        self.inner.close().await
    }

    /// See [`Client::is_terminated`].
    #[inline]
    pub fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }

    /// See [`Client::config`].
    #[inline]
    pub fn config(&self) -> &crate::client::Config {
        self.inner.config()
    }

    /// See [`Client::stats`].
    #[inline]
    pub fn stats(&self) -> crate::client::ClientStats {
        self.inner.stats()
    }

    /// See [`Client::is_connected`].
    #[inline]
    pub fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    /// See [`Client::server_version`].
    #[inline]
    pub fn server_version(&self) -> Option<std::sync::Arc<str>> {
        self.inner.server_version()
    }

    /// See [`Client::on_reconnect`].
    #[inline]
    pub fn on_reconnect(&self) -> ReconnectReceiver {
        self.inner.on_reconnect()
    }

    /// See [`Client::send`].
    #[inline]
    pub async fn send<T: DeserializeOwned>(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<T> {
        self.inner.send(command, retry_on_error).await
    }

    /// See [`Client::send_and_forget`].
    #[inline]
    pub fn send_and_forget(
        &self,
        command: impl Into<Command>,
        retry_on_error: Option<bool>,
    ) -> Result<()> {
        self.inner.send_and_forget(command, retry_on_error)
    }

    /// See [`Client::create_transaction`].
    #[inline]
    pub fn create_transaction(&self) -> Transaction {
        self.inner.create_transaction()
    }

    /// See [`Client::create_pipeline`].
    #[inline]
    pub fn create_pipeline<'a>(&'a self) -> Pipeline<'a> {
        self.inner.create_pipeline()
    }

    /// See [`Client::create_pub_sub`].
    #[inline]
    pub fn create_pub_sub(&self) -> PubSubStream {
        self.inner.create_pub_sub()
    }

    /// See [`Client::create_client_tracking_invalidation_stream`].
    #[inline]
    pub fn create_client_tracking_invalidation_stream(
        &self,
    ) -> Result<ClientTrackingInvalidationStream> {
        self.inner.create_client_tracking_invalidation_stream()
    }
}

impl<'a, R: Response> ClientPreparedCommand<'a, R> for PreparedCommand<'a, &'a ExclusiveClient, R> {
    #[inline]
    fn forget(self) -> Result<()> {
        self.executor
            .inner()
            .send_and_forget(self.command, self.retry_on_error)
    }
}

impl<'a, R: Response + DeserializeOwned + 'a> IntoFuture
    for PreparedCommand<'a, &'a ExclusiveClient, R>
{
    type Output = Result<R>;
    type IntoFuture = CommandFuture<'a, R>;

    #[inline]
    fn into_future(self) -> Self::IntoFuture {
        CommandFuture::new(self.executor.inner(), self.command, self.retry_on_error)
    }
}

impl_shared_command_traits!(ExclusiveClient);

/// The connection is this client's own, so nothing else is stalled while a
/// blocking command holds it.
impl<'a> BlockingCommands<'a> for &'a ExclusiveClient {
    #[inline]
    async fn monitor(self) -> Result<crate::client::MonitorStream> {
        self.inner.monitor_stream().await
    }
}

/// The connection is this client's own, so the watched state cannot be observed
/// or discarded by another handle.
impl<'a> TransactionCommands<'a> for &'a ExclusiveClient {}

impl<'a> PubSubCommands<'a> for &'a ExclusiveClient {
    #[inline]
    async fn subscribe(self, channels: impl Serialize) -> Result<PubSubStream> {
        self.inner.subscribe(channels).await
    }

    #[inline]
    async fn psubscribe(self, patterns: impl Serialize) -> Result<PubSubStream> {
        self.inner.psubscribe(patterns).await
    }

    #[inline]
    async fn ssubscribe(self, shardchannels: impl Serialize) -> Result<PubSubStream> {
        self.inner.ssubscribe(shardchannels).await
    }
}
