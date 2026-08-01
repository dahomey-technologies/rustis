use crate::{Future, Result};
use std::{fmt, sync::Arc};
use tokio::io::{AsyncRead, AsyncWrite};

/// The reading half of a transport handed to the client.
pub type TransportReader = Box<dyn AsyncRead + Send + Unpin>;
/// The writing half of a transport handed to the client.
pub type TransportWriter = Box<dyn AsyncWrite + Send + Unpin>;

/// A source of byte streams the client speaks RESP over, in place of the TCP
/// connection it would otherwise open itself.
///
/// It is asked for a stream at **every** dial — the initial connection and each
/// reconnection — because a stream is consumed by the connection that uses it:
/// a factory that could only produce one would leave the client with nothing to
/// reconnect to, and reconnecting is what the client does whenever the link
/// breaks. This mirrors [`CredentialsProvider`](crate::client::CredentialsProvider),
/// consulted at every handshake for the same reason.
///
/// What this buys, over the [`Standalone`](crate::client::ServerConfig::Standalone)
/// and [`UnixSocket`](crate::client::ServerConfig::UnixSocket) endpoints:
/// * an in-memory pipe ([`tokio::io::duplex`]), so a test drives a server of its
///   own making with no port, no loopback and no ordering left to the network —
///   which is also what lets it run where `bind` is not allowed;
/// * any stream the crate does not know about: a tunnel, an SSH-forwarded
///   channel, a TLS stack configured elsewhere.
///
/// The trait is implemented for any `Fn() -> Future<Output = Result<(TransportReader, TransportWriter)>>`,
/// so a closure is usually all that is needed:
///
/// ```
/// use rustis::client::{Config, CustomTransport, ServerConfig, TransportReader, TransportWriter};
///
/// # fn main() -> rustis::Result<()> {
/// let mut config = Config::default();
/// config.server = ServerConfig::Custom(CustomTransport::new(|| async {
///     let (client_side, server_side) = tokio::io::duplex(4096);
///     // drive `server_side` with a server of your own here
///     drop(server_side);
///     let (reader, writer) = tokio::io::split(client_side);
///     Ok((
///         Box::new(reader) as TransportReader,
///         Box::new(writer) as TransportWriter,
///     ))
/// }));
/// # Ok(())
/// # }
/// ```
///
/// An error returned by the factory fails the connection like a refused socket:
/// the [reconnection policy](crate::client::ReconnectionConfig) retries it with
/// its own backoff.
///
/// The socket options [`Config::keep_alive`](crate::client::Config::keep_alive)
/// and [`Config::no_delay`](crate::client::Config::no_delay) describe a TCP
/// socket and are not applied to a stream coming from here; a factory that
/// wants them sets them on the socket it builds.
pub trait TransportFactory: Send + Sync + 'static {
    /// Yields the two halves of a stream to the server, for the connection
    /// being established.
    fn connect(&self) -> Future<'_, (TransportReader, TransportWriter)>;
}

impl<F, Fut> TransportFactory for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(TransportReader, TransportWriter)>> + Send + 'static,
{
    fn connect(&self) -> Future<'_, (TransportReader, TransportWriter)> {
        Box::pin(self())
    }
}

/// A [`TransportFactory`] as held by
/// [`ServerConfig::Custom`](crate::client::ServerConfig::Custom).
///
/// The wrapper exists so a [`Config`](crate::client::Config) stays `Clone` and
/// `Debug`: a factory is neither, and its `Debug` says only that a transport is
/// injected, never anything about what is behind it.
#[derive(Clone)]
pub struct CustomTransport(Arc<dyn TransportFactory>);

impl CustomTransport {
    /// Wraps `factory`, which may be a closure.
    pub fn new(factory: impl TransportFactory) -> Self {
        Self(Arc::new(factory))
    }

    pub(crate) fn factory(&self) -> &Arc<dyn TransportFactory> {
        &self.0
    }
}

impl fmt::Debug for CustomTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CustomTransport")
    }
}
