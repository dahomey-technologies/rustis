use crate::{ClientError, Error, Result};
#[cfg(feature = "native-tls")]
use native_tls::{Certificate, Identity, Protocol, TlsConnector, TlsConnectorBuilder};
#[cfg(feature = "rustls")]
use std::sync::Arc;
use std::{
    collections::HashMap,
    fmt::{self, Display, Write},
    str::FromStr,
    time::Duration,
};
use url::Url;

const DEFAULT_PORT: u16 = 6379;
const DEFAULT_DATABASE: usize = 0;
const DEFAULT_WAIT_BETWEEN_FAILURES: u64 = 250;
const DEFAULT_CONNECT_TIMEOUT: u64 = 10_000;
const DEFAULT_COMMAND_TIMEOUT: u64 = 0;
const DEFAULT_AUTO_RESUBSCRTBE: bool = true;
const DEFAULT_AUTO_REMONITOR: bool = true;
const DEFAULT_KEEP_ALIVE: Option<Duration> = None;
const DEFAULT_NO_DELAY: bool = true;
const DEFAULT_RETRY_ON_ERROR: bool = false;
const DEFAULT_MAX_COMMAND_ATTEMPTS: usize = 0;
const DEFAULT_MAX_MESSAGES_PER_WAVE: usize = 48;
const DEFAULT_MAX_DISCOVERY_ROUNDS: usize = 10;

/// Sizing and recycling policy for the buffers a connection keeps alive between
/// commands: the read/write framing buffers and the RESP parse tape.
///
/// Every field defaults to the value that was hardcoded before these became
/// configurable, so leaving this alone reproduces the historical behavior
/// exactly. The knobs trade steady-state memory against reallocation: a larger
/// capacity avoids growth on big replies, a smaller one returns memory sooner.
///
/// Both buffers deliberately share one shrink policy — they grow for the same
/// reason (one oversized reply) and should reclaim on the same terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferConfig {
    /// Initial capacity of the read framing buffer, and the target both the read
    /// and write buffers are shrunk back to once they have grown oversized.
    ///
    /// These are one parameter, not two: starting below the shrink target would
    /// make the first large reply grow the buffer only to have it reclaimed.
    ///
    /// The default is 64 KiB.
    pub read_capacity: usize,
    /// Capacity a recycled parse-tape buffer is reset to once it has been
    /// oversized and quiet for long enough. 64 KiB = 8192 tape nodes, deep
    /// enough that a normal reply's tape never reallocates.
    ///
    /// The default is 64 KiB.
    pub tape_capacity: usize,
    /// A buffer is only considered for shrinking once it exceeds this multiple
    /// of its target, so a workload alternating large and small replies does not
    /// reallocate every cycle.
    ///
    /// The default is `8`.
    pub shrink_factor: usize,
    /// Consecutive quiet observations required before actually paying for the
    /// shrink realloc.
    ///
    /// The default is `16`.
    pub shrink_hysteresis: usize,
}

impl BufferConfig {
    /// The default policy, usable in a `const` context.
    pub const DEFAULT: Self = Self {
        read_capacity: 64 * 1024,
        tape_capacity: 64 * 1024,
        shrink_factor: 8,
        shrink_hysteresis: 16,
    };

    fn validate(&self) -> Result<()> {
        // Each of these is a capacity, a multiplier or a streak length whose zero
        // value does not soften the policy but removes it.
        if self.read_capacity == 0 {
            return Err(invalid_config(
                "buffers.read_capacity must be greater than 0",
            ));
        }
        if self.tape_capacity == 0 {
            return Err(invalid_config(
                "buffers.tape_capacity must be greater than 0",
            ));
        }
        if self.shrink_factor == 0 {
            return Err(invalid_config(
                "buffers.shrink_factor must be greater than 0",
            ));
        }
        if self.shrink_hysteresis == 0 {
            return Err(invalid_config(
                "buffers.shrink_hysteresis must be greater than 0",
            ));
        }
        Ok(())
    }
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Limits the RESP parser enforces against hostile or corrupt server input.
///
/// These bound pathology, they do not police normal use: every default is
/// generous enough for any legitimate reply, and is the value that was hardcoded
/// before these became configurable. Raising one widens the resources a single
/// reply can command; lowering one can reject replies a real server sends.
///
/// A frame breaching any limit fails the connection with the matching
/// [`ClientError`](crate::ClientError) rather than being reported as a truncated
/// read, so the streaming decoder never waits for bytes that will never come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespLimits {
    /// Maximum collection-nesting depth accepted before a frame is rejected with
    /// [`MaxNestingDepthExceeded`](crate::ClientError::MaxNestingDepthExceeded).
    ///
    /// RESP replies are shallow in practice — a handful of levels for the
    /// deepest cluster and stream introspection commands — so this stops a
    /// crafted `*1\r\n*1\r\n…` reply from driving the parser into a stack
    /// overflow, which unlike a panic is not catchable and aborts the whole
    /// process. The element loop is iterative, so this bounds the parser's
    /// explicit stack (and the recursion left in attribute skipping) rather than
    /// the call stack.
    ///
    /// The default is `128`.
    pub max_nesting_depth: usize,
    /// Maximum byte length accepted for a single bulk string, bulk error or
    /// verbatim string, checked against the declared header before the payload
    /// is trusted; breaching it raises
    /// [`BulkLengthTooLarge`](crate::ClientError::BulkLengthTooLarge).
    ///
    /// Matches Redis's own `proto-max-bulk-len` default. Raise it only if the
    /// server's is also raised.
    ///
    /// The default is 512 MiB.
    pub max_bulk_length: usize,
    /// Maximum number of elements accepted in a single collection — array, set,
    /// push or map, counted after the map key/value doubling; breaching it raises
    /// [`CollectionLengthTooLarge`](crate::ClientError::CollectionLengthTooLarge).
    ///
    /// Bounds an attacker-controlled loop count and the buffer pre-reservation
    /// derived from it.
    ///
    /// The default is 128 Mi elements.
    pub max_collection_length: usize,
}

impl RespLimits {
    /// The default limits, usable in a `const` context.
    pub const DEFAULT: Self = Self {
        max_nesting_depth: 128,
        max_bulk_length: 512 * 1024 * 1024,
        max_collection_length: 128 * 1024 * 1024,
    };

    fn validate(&self) -> Result<()> {
        // A zero limit rejects every collection or every bulk value, which is not
        // a stricter client but an unusable one.
        if self.max_nesting_depth == 0 {
            return Err(invalid_config(
                "limits.max_nesting_depth must be greater than 0",
            ));
        }
        if self.max_bulk_length == 0 {
            return Err(invalid_config(
                "limits.max_bulk_length must be greater than 0",
            ));
        }
        if self.max_collection_length == 0 {
            return Err(invalid_config(
                "limits.max_collection_length must be greater than 0",
            ));
        }
        Ok(())
    }
}

impl Default for RespLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[inline]
fn invalid_config(message: &'static str) -> Error {
    Error::Client(ClientError::InvalidConfig(message))
}

type Uri<'a> = (
    &'a str,
    Option<&'a str>,
    Option<&'a str>,
    Vec<(&'a str, u16)>,
    Vec<&'a str>,
    Option<HashMap<String, String>>,
);

/// Configuration options for a [`client`](crate::client::Client)
/// or a [`pooled client`](crate::client::PooledClientManager)
#[derive(Clone)]
pub struct Config {
    /// Connection server configuration (standalone, sentinel, or cluster)
    pub server: ServerConfig,
    /// An optional ACL username for authentication.
    ///
    /// See [`ACL`](https://redis.io/docs/management/security/acl/)
    pub username: Option<String>,
    /// An optional password for authentication.
    ///
    /// The password could be either coupled with an ACL username either used alone.
    ///
    /// See:
    /// * [`ACL`](https://redis.io/docs/management/security/acl/)
    /// * [`Authentication`](https://redis.io/docs/management/security/#authentication)
    pub password: Option<String>,
    /// The default database for this connection.
    ///
    /// If `database` is not set to `0`, a [`SELECT`](https://redis.io/commands/select/)
    /// command will be automatically issued at connection or reconnection.
    pub database: usize,
    /// An optional TLS configuration.
    #[cfg_attr(docsrs, doc(cfg(any(feature = "native-tls", feature = "rustls"))))]
    #[cfg(any(feature = "native-tls", feature = "rustls"))]
    pub tls_config: Option<TlsConfig>,
    /// The time to attempt a connection before timing out. The default is 10 seconds
    pub connect_timeout: Duration,
    /// If a command does not return a reply within a set number of milliseconds,
    /// a timeout error will be thrown.
    ///
    /// If set to 0, no timeout is apply
    ///
    /// The default is 0
    pub command_timeout: Duration,
    /// When the client reconnects, channels subscribed in the previous connection will be
    /// resubscribed automatically if `auto_resubscribe` is `true`.
    ///
    /// The default is `true`
    pub auto_resubscribe: bool,
    /// When the client reconnects, if in `monitor` mode, the
    /// [`monitor`](crate::commands::BlockingCommands::monitor) command
    /// will be resent automatically
    ///
    /// The default is `true`
    pub auto_remonitor: bool,
    /// Set the name of the connection to make it easier to identity the connection in client list.
    ///
    /// See [`client_setname`](crate::commands::ConnectionCommands::client_setname)
    pub connection_name: String,
    /// Enable/disable keep-alive functionality (default `None`)
    ///
    /// See [`TcpKeepAlive::with_time`](https://docs.rs/socket2/latest/socket2/struct.TcpKeepalive.html#method.with_time)
    pub keep_alive: Option<Duration>,
    /// Enable/disable the use of Nagle's algorithm (default `true`)
    ///
    /// See [`TcpStream::set_nodelay`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html#method.set_nodelay)    
    pub no_delay: bool,
    /// Defines the default strategy for retries on network error (default `false`):
    /// * `true` - retry sending the command/batch of commands on network error
    /// * `false` - do not retry sending the command/batch of commands on network error
    ///
    /// This strategy can be overriden for each command/batch
    /// of commands in the following functions:
    /// * [`PreparedCommand::retry_on_error`](crate::client::PreparedCommand::retry_on_error)
    /// * [`Pipeline::retry_on_error`](crate::client::Pipeline::retry_on_error)
    /// * [`Transaction::retry_on_error`](crate::client::Transaction::retry_on_error)
    /// * [`Client::send`](crate::client::Client::send)
    /// * [`Client::send_and_forget`](crate::client::Client::send_and_forget)
    /// * [`Client::send_batch`](crate::client::Client::send_batch)
    pub retry_on_error: bool,
    /// Reconnection policy configuration (Constant, Linear or Exponential)
    pub reconnection: ReconnectionConfig,
    /// Maximum number of times a single command/batch may be attempted before it
    /// is failed with [`ClientError::MaxCommandAttemptsReached`](crate::ClientError::MaxCommandAttemptsReached)
    /// instead of being retried again.
    ///
    /// Retries happen on cluster `ASK`/`MOVED` redirections and on reconnection
    /// replay of `retry_on_error` commands. The connection-level
    /// [`reconnection`](Self::reconnection) cap does not bound them, so a command
    /// caught in a pathological redirect or reconnect loop would otherwise be
    /// replayed forever.
    ///
    /// The default is `0`, meaning unlimited (the historical behavior).
    pub max_command_attempts: usize,
    /// Sizing and recycling policy for the connection's internal buffers.
    pub buffers: BufferConfig,
    /// Limits the RESP parser enforces against hostile or corrupt server input.
    pub limits: RespLimits,
    /// Maximum number of queued commands the network task writes in one wave
    /// before flushing, instead of draining its whole channel into a single
    /// write. Capping the wave lets the first commands reach the server while
    /// the next ones are still being collected, which removes the convoy effect
    /// under high concurrency.
    ///
    /// The cap only fires above its own value of in-flight commands, so lower
    /// concurrencies are unaffected whatever it is set to. The optimum is flat
    /// between 32 and 128 and only mildly concurrency-dependent; what matters is
    /// that it stays *below* the in-flight concurrency, otherwise it never fires.
    ///
    /// The default is `48`.
    pub max_messages_per_wave: usize,
    /// Test-only hook to observe and inject retry reasons in the send batch.
    ///
    /// Only present in debug builds; it carries no cost in release builds.
    #[cfg(test)]
    pub(crate) send_batch_test_hook: Option<crate::network::SendBatchTestHook>,
    /// Test-only hook to make the cluster topology-refresh failure path
    /// observable (simulate a node vanishing while requests are in flight).
    ///
    /// Only present in test builds; it carries no cost in release builds.
    #[cfg(test)]
    pub(crate) cluster_test_hook: Option<crate::network::ClusterTestHook>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Config");
        s.field("server", &self.server)
            .field("username", &self.username)
            // never leak the password in clear text
            .field("password", &self.password.as_ref().map(|_| "***"))
            .field("database", &self.database);
        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        s.field("tls_config", &self.tls_config);
        s.field("connect_timeout", &self.connect_timeout)
            .field("command_timeout", &self.command_timeout)
            .field("auto_resubscribe", &self.auto_resubscribe)
            .field("auto_remonitor", &self.auto_remonitor)
            .field("connection_name", &self.connection_name)
            .field("keep_alive", &self.keep_alive)
            .field("no_delay", &self.no_delay)
            .field("retry_on_error", &self.retry_on_error)
            .field("reconnection", &self.reconnection)
            .field("max_command_attempts", &self.max_command_attempts)
            .field("buffers", &self.buffers)
            .field("limits", &self.limits)
            .field("max_messages_per_wave", &self.max_messages_per_wave)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Default::default(),
            username: Default::default(),
            password: Default::default(),
            database: Default::default(),
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            tls_config: Default::default(),
            connect_timeout: Duration::from_millis(DEFAULT_CONNECT_TIMEOUT),
            command_timeout: Duration::from_millis(DEFAULT_COMMAND_TIMEOUT),
            auto_resubscribe: DEFAULT_AUTO_RESUBSCRTBE,
            auto_remonitor: DEFAULT_AUTO_REMONITOR,
            connection_name: String::from(""),
            keep_alive: DEFAULT_KEEP_ALIVE,
            no_delay: DEFAULT_NO_DELAY,
            retry_on_error: DEFAULT_RETRY_ON_ERROR,
            reconnection: Default::default(),
            max_command_attempts: DEFAULT_MAX_COMMAND_ATTEMPTS,
            buffers: Default::default(),
            limits: Default::default(),
            max_messages_per_wave: DEFAULT_MAX_MESSAGES_PER_WAVE,
            #[cfg(test)]
            send_batch_test_hook: None,
            #[cfg(test)]
            cluster_test_hook: None,
        }
    }
}

impl FromStr for Config {
    type Err = Error;

    /// Build a config from an URI or a standard address format `host`:`port`
    fn from_str(str: &str) -> Result<Config> {
        if let Some(config) = Self::parse_uri(str) {
            Ok(config)
        } else if let Some(addr) = Self::parse_addr(str) {
            addr.into_config()
        } else {
            Err(Error::Client(ClientError::ConfigParseError))
        }
    }
}

impl Config {
    /// Build a config from an URI in the format `redis[s]://[[username]:password@]host[:port]/[database]`
    pub fn from_uri(uri: Url) -> Result<Config> {
        Self::from_str(uri.as_str())
    }

    /// Checks the tuning knobs for values that would disable behavior rather
    /// than tune it, returning
    /// [`ClientError::InvalidConfig`](crate::ClientError::InvalidConfig) naming
    /// the offending one.
    ///
    /// Called for you when a client connects. It is public so a config assembled
    /// programmatically can be checked up front rather than at connect time.
    ///
    /// This validates *coherence*, not taste: a capacity of 1 byte or a limit of
    /// 1 element is accepted, because bad-but-working values are the caller's
    /// call. Only values that remove a behavior outright are rejected — the
    /// fields are public, so nothing stops a caller from zeroing one after
    /// [`Default`] filled it in.
    pub fn validate(&self) -> Result<()> {
        self.buffers.validate()?;
        self.limits.validate()?;
        // A wave of zero flushes before any message is queued, so the network
        // task would spin without ever writing.
        if self.max_messages_per_wave == 0 {
            return Err(invalid_config(
                "max_messages_per_wave must be greater than 0",
            ));
        }
        if let ServerConfig::Sentinel(sentinel_config) = &self.server {
            // Zero rounds gives up before contacting any Sentinel instance.
            if sentinel_config.max_discovery_rounds == 0 {
                return Err(invalid_config(
                    "sentinel max_discovery_rounds must be greater than 0",
                ));
            }
        }
        Ok(())
    }

    /// Parse address in the standard format `host`:`port`, including bracketed
    /// IPv6 (`[::1]` / `[::1]:6379`) whose host part itself contains colons.
    fn parse_addr(str: &str) -> Option<(&str, u16)> {
        // Bracketed IPv6: the host is inside `[...]`, an optional `:port` follows.
        if let Some(rest) = str.strip_prefix('[') {
            let (host, after) = rest.split_once(']')?;
            return match after {
                "" => Some((host, DEFAULT_PORT)),
                _ => {
                    let port = after.strip_prefix(':')?;
                    Some((host, port.parse::<u16>().ok()?))
                }
            };
        }

        let mut iter = str.split(':');

        match (iter.next(), iter.next(), iter.next()) {
            (Some(host), Some(port), None) => {
                if let Ok(port) = port.parse::<u16>() {
                    Some((host, port))
                } else {
                    None
                }
            }
            (Some(host), None, None) => Some((host, DEFAULT_PORT)),
            _ => None,
        }
    }

    fn parse_uri(uri: &str) -> Option<Config> {
        let (scheme, username, password, hosts, path_segments, mut query) =
            Self::break_down_uri(uri)?;
        let mut hosts = hosts;
        let mut path_segments = path_segments.into_iter();

        enum ServerType {
            Standalone,
            Sentinel,
            Cluster,
        }

        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        let (tls_config, server_type) = match scheme {
            "redis" => (None, ServerType::Standalone),
            "rediss" => (Some(TlsConfig::default()), ServerType::Standalone),
            "redis+sentinel" | "redis-sentinel" => (None, ServerType::Sentinel),
            "rediss+sentinel" | "rediss-sentinel" => {
                (Some(TlsConfig::default()), ServerType::Sentinel)
            }
            "redis+cluster" | "redis-cluster" => (None, ServerType::Cluster),
            "rediss+cluster" | "rediss-cluster" => {
                (Some(TlsConfig::default()), ServerType::Cluster)
            }
            _ => {
                return None;
            }
        };

        #[cfg(not(any(feature = "native-tls", feature = "rustls")))]
        let server_type = match scheme {
            "redis" => ServerType::Standalone,
            "redis+sentinel" | "redis-sentinel" => ServerType::Sentinel,
            "redis+cluster" | "redis-cluster" => ServerType::Cluster,
            _ => {
                return None;
            }
        };

        let server = match server_type {
            ServerType::Standalone => {
                if hosts.len() > 1 {
                    return None;
                } else {
                    let (host, port) = hosts.pop()?;
                    ServerConfig::Standalone {
                        host: host.to_owned(),
                        port,
                    }
                }
            }
            ServerType::Sentinel => {
                let instances = hosts
                    .iter()
                    .map(|(host, port)| ((*host).to_owned(), *port))
                    .collect::<Vec<_>>();

                let service_name = match path_segments.next() {
                    Some(service_name) => service_name.to_owned(),
                    None => {
                        return None;
                    }
                };

                let mut sentinel_config = SentinelConfig {
                    instances,
                    service_name,
                    ..Default::default()
                };

                if let Some(ref mut query) = query {
                    if let Some(millis) = query.remove("wait_between_failures")
                        && let Ok(millis) = millis.parse::<u64>()
                    {
                        sentinel_config.wait_between_failures = Duration::from_millis(millis);
                    }

                    sentinel_config.username = query.remove("sentinel_username");
                    sentinel_config.password = query.remove("sentinel_password");
                }

                ServerConfig::Sentinel(sentinel_config)
            }
            ServerType::Cluster => {
                let nodes = hosts
                    .iter()
                    .map(|(host, port)| ((*host).to_owned(), *port))
                    .collect::<Vec<_>>();

                ServerConfig::Cluster(ClusterConfig { nodes })
            }
        };

        let database = match path_segments.next() {
            Some(database) => match database.parse::<usize>() {
                Ok(database) => database,
                Err(_) => {
                    return None;
                }
            },
            None => DEFAULT_DATABASE,
        };

        let mut config = Config {
            server,
            // Credentials are percent-encoded in a URI (a password may legally
            // contain `@`, `:`, `/`, `%`). Decode them so the server receives the
            // literal secret, as standard clients do — otherwise `p%40ss` would be
            // sent verbatim and authentication would fail.
            username: username.map(percent_decode),
            password: password.map(percent_decode),
            database,
            #[cfg(any(feature = "native-tls", feature = "rustls"))]
            tls_config,
            ..Default::default()
        };

        if let Some(ref mut query) = query {
            if let Some(millis) = query.remove("connect_timeout")
                && let Ok(millis) = millis.parse::<u64>()
            {
                config.connect_timeout = Duration::from_millis(millis);
            }

            if let Some(millis) = query.remove("command_timeout")
                && let Ok(millis) = millis.parse::<u64>()
            {
                config.command_timeout = Duration::from_millis(millis);
            }

            if let Some(auto_resubscribe) = query.remove("auto_resubscribe")
                && let Ok(auto_resubscribe) = auto_resubscribe.parse::<bool>()
            {
                config.auto_resubscribe = auto_resubscribe;
            }

            if let Some(auto_remonitor) = query.remove("auto_remonitor")
                && let Ok(auto_remonitor) = auto_remonitor.parse::<bool>()
            {
                config.auto_remonitor = auto_remonitor;
            }

            if let Some(connection_name) = query.remove("connection_name") {
                config.connection_name = connection_name;
            }

            if let Some(keep_alive) = query.remove("keep_alive")
                && let Ok(keep_alive) = keep_alive.parse::<u64>()
            {
                config.keep_alive = Some(Duration::from_millis(keep_alive));
            }

            if let Some(no_delay) = query.remove("no_delay")
                && let Ok(no_delay) = no_delay.parse::<bool>()
            {
                config.no_delay = no_delay;
            }

            if let Some(retry_on_error) = query.remove("retry_on_error")
                && let Ok(retry_on_error) = retry_on_error.parse::<bool>()
            {
                config.retry_on_error = retry_on_error;
            }

            if let Some(max_command_attempts) = query.remove("max_command_attempts")
                && let Ok(max_command_attempts) = max_command_attempts.parse::<usize>()
            {
                config.max_command_attempts = max_command_attempts;
            }
        }

        Some(config)
    }

    /// break down an uri in a tuple (scheme, username, password, hosts, path_segments)
    fn break_down_uri<'a>(uri: &'a str) -> Option<Uri<'a>> {
        let end_of_scheme = match uri.find("://") {
            Some(index) => index,
            None => {
                return None;
            }
        };

        let scheme = &uri[..end_of_scheme];

        let after_scheme = &uri[end_of_scheme + 3..];

        let (before_query, query) = match after_scheme.find('?') {
            Some(index) => match Self::exclusive_split_at(after_scheme, index) {
                (Some(before_query), after_query) => (before_query, after_query),
                _ => {
                    return None;
                }
            },
            None => (after_scheme, None),
        };

        let (authority, path) = match before_query.find('/') {
            Some(index) => match Self::exclusive_split_at(before_query, index) {
                (Some(authority), path) => (authority, path),
                _ => {
                    return None;
                }
            },
            None => (before_query, None),
        };

        let (user_info, hosts) = match authority.rfind('@') {
            Some(index) => {
                // if '@' is in the host section, it MUST be interpreted as a request for
                // authentication, even if the credentials are empty.
                let (user_info, hosts) = Self::exclusive_split_at(authority, index);
                match hosts {
                    Some(hosts) => (user_info, hosts),
                    None => {
                        // missing hosts
                        return None;
                    }
                }
            }
            None => (None, authority),
        };

        let (username, password) = match user_info {
            Some(user_info) => match user_info.find(':') {
                Some(index) => match Self::exclusive_split_at(user_info, index) {
                    (username, None) => (username, Some("")),
                    (username, password) => (username, password),
                },
                None => {
                    // username without password is not accepted
                    return None;
                }
            },
            None => (None, None),
        };

        let hosts = hosts
            .split(',')
            .map(Self::parse_addr)
            .collect::<Option<Vec<_>>>();
        let hosts = hosts?;

        let path_segments = match path {
            Some(path) => path.split('/').collect::<Vec<_>>(),
            None => Vec::new(),
        };

        let query = match query.map(|q| {
            q.split('&')
                .map(|s| s.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
                .collect::<Option<HashMap<String, String>>>()
        }) {
            Some(Some(query)) => Some(query),
            Some(None) => return None,
            None => None,
        };

        Some((scheme, username, password, hosts, path_segments, query))
    }

    /// Splits a string into a section before a given index and a section exclusively after the index.
    /// Empty portions are returned as `None`.
    fn exclusive_split_at(s: &str, i: usize) -> (Option<&str>, Option<&str>) {
        let (l, r) = s.split_at(i);

        let lout = if !l.is_empty() { Some(l) } else { None };
        let rout = if r.len() > 1 { Some(&r[1..]) } else { None };

        (lout, rout)
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(any(feature = "native-tls", feature = "rustls"))]
        if self.tls_config.is_some() {
            match &self.server {
                ServerConfig::Standalone { host: _, port: _ } => f.write_str("rediss://")?,
                ServerConfig::Sentinel(_) => f.write_str("rediss+sentinel://")?,
                ServerConfig::Cluster(_) => f.write_str("rediss+cluster://")?,
            }
        } else {
            match &self.server {
                ServerConfig::Standalone { host: _, port: _ } => f.write_str("redis://")?,
                ServerConfig::Sentinel(_) => f.write_str("redis+sentinel://")?,
                ServerConfig::Cluster(_) => f.write_str("redis+cluster://")?,
            }
        }

        #[cfg(not(any(feature = "native-tls", feature = "rustls")))]
        match &self.server {
            ServerConfig::Standalone { host: _, port: _ } => f.write_str("redis://")?,
            ServerConfig::Sentinel(_) => f.write_str("redis+sentinel://")?,
            ServerConfig::Cluster(_) => f.write_str("redis+cluster://")?,
        }

        if let Some(username) = &self.username {
            f.write_str(username)?;
        }

        if self.password.is_some() {
            // never leak the password in clear text (e.g. when logging a config)
            f.write_str(":***@")?;
        }

        match &self.server {
            ServerConfig::Standalone { host, port } => {
                f.write_str(host)?;
                if *port != DEFAULT_PORT {
                    f.write_char(':')?;
                    f.write_str(&port.to_string())?;
                }
            }
            ServerConfig::Sentinel(SentinelConfig {
                instances,
                service_name,
                wait_between_failures: _,
                max_discovery_rounds: _,
                password: _,
                username: _,
            }) => {
                f.write_str(
                    &instances
                        .iter()
                        .map(|(host, port)| format!("{host}:{port}"))
                        .collect::<Vec<String>>()
                        .join(","),
                )?;
                f.write_char('/')?;
                f.write_str(service_name)?;
            }
            ServerConfig::Cluster(ClusterConfig { nodes }) => {
                f.write_str(
                    &nodes
                        .iter()
                        .map(|(host, port)| format!("{host}:{port}"))
                        .collect::<Vec<String>>()
                        .join(","),
                )?;
            }
        }

        if self.database > 0 {
            f.write_char('/')?;
            f.write_str(&self.database.to_string())?;
        }

        // query

        let mut query_separator = false;

        let connect_timeout = self.connect_timeout.as_millis() as u64;
        if connect_timeout != DEFAULT_CONNECT_TIMEOUT {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("connect_timeout={connect_timeout}"))?;
        }

        let command_timeout = self.command_timeout.as_millis() as u64;
        if command_timeout != DEFAULT_COMMAND_TIMEOUT {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("command_timeout={command_timeout}"))?;
        }

        if self.auto_resubscribe != DEFAULT_AUTO_RESUBSCRTBE {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("auto_resubscribe={}", self.auto_resubscribe))?;
        }

        if self.auto_remonitor != DEFAULT_AUTO_REMONITOR {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("auto_remonitor={}", self.auto_remonitor))?;
        }

        if !self.connection_name.is_empty() {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("connection_name={}", self.connection_name))?;
        }

        if let Some(keep_alive) = self.keep_alive {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("keep_alive={}", keep_alive.as_millis()))?;
        }

        if self.no_delay != DEFAULT_NO_DELAY {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("no_delay={}", self.no_delay))?;
        }

        if self.retry_on_error != DEFAULT_RETRY_ON_ERROR {
            if !query_separator {
                query_separator = true;
                f.write_char('?')?;
            } else {
                f.write_char('&')?;
            }
            f.write_fmt(format_args!("retry_on_error={}", self.retry_on_error))?;
        }

        if let ServerConfig::Sentinel(SentinelConfig {
            instances: _,
            service_name: _,
            wait_between_failures: wait_beetween_failures,
            max_discovery_rounds: _,
            password,
            username,
        }) = &self.server
        {
            let wait_between_failures = wait_beetween_failures.as_millis() as u64;
            if wait_between_failures != DEFAULT_WAIT_BETWEEN_FAILURES {
                if !query_separator {
                    query_separator = true;
                    f.write_char('?')?;
                } else {
                    f.write_char('&')?;
                }
                f.write_fmt(format_args!(
                    "wait_between_failures={wait_between_failures}"
                ))?;
            }
            if let Some(username) = username {
                if !query_separator {
                    query_separator = true;
                    f.write_char('?')?;
                } else {
                    f.write_char('&')?;
                }
                f.write_str("sentinel_username=")?;
                f.write_str(username)?;
            }
            if password.is_some() {
                if !query_separator {
                    f.write_char('?')?;
                } else {
                    f.write_char('&')?;
                }
                // never leak the password in clear text
                f.write_str("sentinel_password=***")?;
            }
        }

        Ok(())
    }
}

/// Configuration for connecting to a Redis server
#[derive(Debug, Clone)]
pub enum ServerConfig {
    /// Configuration for connecting to a standalone server (no master-replica, no cluster)
    Standalone {
        /// The hostname or IP address of the Redis server.
        host: String,
        /// The port on which the Redis server is listening.
        port: u16,
    },
    /// Configuration for connecting to a Redis server via [`Sentinel`](https://redis.io/docs/management/sentinel/)
    Sentinel(SentinelConfig),
    /// Configuration for connecting to a Redis [`Cluster`](https://redis.io/docs/management/scaling/)
    Cluster(ClusterConfig),
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig::Standalone {
            host: "127.0.0.1".to_owned(),
            port: 6379,
        }
    }
}

/// Configuration for connecting to a Redis server via [`Sentinel`](https://redis.io/docs/management/sentinel/)
#[derive(Clone)]
pub struct SentinelConfig {
    /// An array of `(host, port)` tuples for each known sentinel instance.
    pub instances: Vec<(String, u16)>,

    /// The service name
    pub service_name: String,

    /// Waiting time after failing before connecting to the next Sentinel instance (default 250ms).
    pub wait_between_failures: Duration,

    /// Maximum number of full discovery rounds before giving up.
    ///
    /// One round tries every known instance in turn. The cap bounds an otherwise
    /// unbounded restart loop: a stale Sentinel persistently announcing a
    /// non-master instance would spin forever, one
    /// [`wait_between_failures`](Self::wait_between_failures) apart. Raise it for
    /// a cluster whose failovers routinely outlast ten rounds.
    ///
    /// The default is `10`.
    pub max_discovery_rounds: usize,

    /// Sentinel username
    pub username: Option<String>,

    /// Sentinel password
    pub password: Option<String>,
}

impl fmt::Debug for SentinelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SentinelConfig")
            .field("instances", &self.instances)
            .field("service_name", &self.service_name)
            .field("wait_between_failures", &self.wait_between_failures)
            .field("max_discovery_rounds", &self.max_discovery_rounds)
            .field("username", &self.username)
            // never leak the password in clear text
            .field("password", &self.password.as_ref().map(|_| "***"))
            .finish()
    }
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            instances: Default::default(),
            service_name: Default::default(),
            wait_between_failures: Duration::from_millis(DEFAULT_WAIT_BETWEEN_FAILURES),
            max_discovery_rounds: DEFAULT_MAX_DISCOVERY_ROUNDS,
            password: None,
            username: None,
        }
    }
}

/// Configuration for connecting to a Redis [`Cluster`](https://redis.io/docs/management/scaling/)
#[derive(Debug, Clone, Default)]
pub struct ClusterConfig {
    /// An array of `(host, port)` tuples for each known cluster node.
    pub nodes: Vec<(String, u16)>,
}

/// Config for TLS.
///
/// See [rustls::client::ClientConfig](https://docs.rs/rustls/latest/rustls/client/struct.ClientConfig.html) documentation
#[cfg(feature = "rustls")]
#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub rustls_config: Arc<rustls::ClientConfig>,
}

#[cfg(feature = "rustls")]
impl Default for TlsConfig {
    fn default() -> Self {
        let root_store =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let rustls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        Self {
            rustls_config: Arc::new(rustls_config),
        }
    }
}

/// Config for TLS.
///
/// See [TlsConnectorBuilder](https://docs.rs/tokio-native-tls/latest/tokio_native_tls/native_tls/struct.TlsConnectorBuilder.html) documentation
#[cfg(feature = "native-tls")]
#[derive(Clone)]
pub struct TlsConfig {
    identity: Option<Identity>,
    root_certificates: Option<Vec<Certificate>>,
    min_protocol_version: Option<Protocol>,
    max_protocol_version: Option<Protocol>,
    disable_built_in_roots: bool,
    danger_accept_invalid_certs: bool,
    danger_accept_invalid_hostnames: bool,
    use_sni: bool,
}

#[cfg(feature = "native-tls")]
impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            identity: None,
            root_certificates: None,
            // TLS 1.0/1.1 are deprecated by RFC 8996; default to TLS 1.2
            min_protocol_version: Some(Protocol::Tlsv12),
            max_protocol_version: None,
            disable_built_in_roots: false,
            danger_accept_invalid_certs: false,
            danger_accept_invalid_hostnames: false,
            use_sni: true,
        }
    }
}

#[cfg(feature = "native-tls")]
impl std::fmt::Debug for TlsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsConfig")
            .field("min_protocol_version", &self.min_protocol_version)
            .field("max_protocol_version", &self.max_protocol_version)
            .field("disable_built_in_roots", &self.disable_built_in_roots)
            .field(
                "danger_accept_invalid_certs",
                &self.danger_accept_invalid_certs,
            )
            .field(
                "danger_accept_invalid_hostnames",
                &self.danger_accept_invalid_hostnames,
            )
            .field("use_sni", &self.use_sni)
            .finish()
    }
}

#[cfg(feature = "native-tls")]
impl TlsConfig {
    pub fn identity(&mut self, identity: Identity) -> &mut Self {
        self.identity = Some(identity);
        self
    }

    pub fn root_certificates(&mut self, root_certificates: Vec<Certificate>) -> &mut Self {
        self.root_certificates = Some(root_certificates);
        self
    }

    pub fn min_protocol_version(&mut self, min_protocol_version: Protocol) -> &mut Self {
        self.min_protocol_version = Some(min_protocol_version);
        self
    }

    pub fn max_protocol_version(&mut self, max_protocol_version: Protocol) -> &mut Self {
        self.max_protocol_version = Some(max_protocol_version);
        self
    }

    pub fn disable_built_in_roots(&mut self, disable_built_in_roots: bool) -> &mut Self {
        self.disable_built_in_roots = disable_built_in_roots;
        self
    }

    pub fn danger_accept_invalid_certs(&mut self, danger_accept_invalid_certs: bool) -> &mut Self {
        self.danger_accept_invalid_certs = danger_accept_invalid_certs;
        self
    }

    pub fn use_sni(&mut self, use_sni: bool) -> &mut Self {
        self.use_sni = use_sni;
        self
    }

    pub fn danger_accept_invalid_hostnames(
        &mut self,
        danger_accept_invalid_hostnames: bool,
    ) -> &mut Self {
        self.danger_accept_invalid_hostnames = danger_accept_invalid_hostnames;
        self
    }

    pub fn into_tls_connector_builder(&self) -> TlsConnectorBuilder {
        let mut builder = TlsConnector::builder();

        if let Some(root_certificates) = &self.root_certificates {
            for root_certificate in root_certificates {
                builder.add_root_certificate(root_certificate.clone());
            }
        }

        builder.min_protocol_version(self.min_protocol_version);
        builder.max_protocol_version(self.max_protocol_version);
        builder.disable_built_in_roots(self.disable_built_in_roots);
        builder.danger_accept_invalid_certs(self.danger_accept_invalid_certs);
        builder.danger_accept_invalid_hostnames(self.danger_accept_invalid_hostnames);
        builder.use_sni(self.use_sni);

        builder
    }
}

/// A value-to-[`Config`](crate::client::Config) conversion that consumes the input value.
///
/// This allows the `connect` associated function of the [`client`](crate::client::Client),
/// or [`pooled client`](crate::client::PooledClientManager)
/// to accept connection information in a range of different formats.
pub trait IntoConfig {
    /// Converts this type into a [`Config`](crate::client::Config).
    fn into_config(self) -> Result<Config>;
}

impl IntoConfig for Config {
    fn into_config(self) -> Result<Config> {
        Ok(self)
    }
}

impl<T: Into<String>> IntoConfig for (T, u16) {
    fn into_config(self) -> Result<Config> {
        Ok(Config {
            server: ServerConfig::Standalone {
                host: self.0.into(),
                port: self.1,
            },
            ..Default::default()
        })
    }
}

impl IntoConfig for &str {
    fn into_config(self) -> Result<Config> {
        Config::from_str(self)
    }
}

impl IntoConfig for String {
    fn into_config(self) -> Result<Config> {
        Config::from_str(&self)
    }
}

impl IntoConfig for Url {
    fn into_config(self) -> Result<Config> {
        Config::from_uri(self)
    }
}

/// The type of reconnection policy to use. This will apply to every connection used by the client.
/// This code has been mostly inpisred by [fred ReconnectPolicy](https://docs.rs/fred/latest/fred/types/enum.ReconnectPolicy.html)
#[derive(Debug, Clone)]
pub enum ReconnectionConfig {
    /// Wait a constant amount of time between reconnection attempts, in ms.
    Constant {
        /// Maximum number of attemps, set `0` to retry forever.
        max_attempts: u32,
        /// Delay in ms to wait between reconnection attempts
        delay: u32,
        /// Add jitter in ms to each delay
        jitter: u32,
    },
    /// Backoff reconnection attempts linearly, adding `delay` each time.
    Linear {
        /// Maximum number of attemps, set `0` to retry forever.
        max_attempts: u32,
        /// Maximum delay in ms
        max_delay: u32,
        /// Delay in ms to add to the total waiting time at each attemp
        delay: u32,
        /// Add jitter in ms to each delay
        jitter: u32,
    },
    /// Backoff reconnection attempts exponentially, multiplying the last delay by `multiplicative_factor` each time.
    ///
    /// see <https://en.wikipedia.org/wiki/Exponential_backoff>
    Exponential {
        /// Maximum number of attemps, set `0` to retry forever.
        max_attempts: u32,
        /// Minimum delay in ms
        min_delay: u32,
        /// Maximum delay in ms
        max_delay: u32,
        // multiplicative factor
        multiplicative_factor: u32,
        /// Add jitter in ms to each delay
        jitter: u32,
    },
}

/// The default amount of jitter when waiting to reconnect.
const DEFAULT_JITTER_MS: u32 = 100;
const DEFAULT_DELAY_MS: u32 = 1000;

impl Default for ReconnectionConfig {
    fn default() -> Self {
        Self::Constant {
            max_attempts: 0,
            delay: DEFAULT_DELAY_MS,
            jitter: DEFAULT_JITTER_MS,
        }
    }
}

impl ReconnectionConfig {
    /// Create a new reconnect policy with a constant backoff.
    pub fn new_constant(max_attempts: u32, delay: u32) -> Self {
        Self::Constant {
            max_attempts,
            delay,
            jitter: DEFAULT_JITTER_MS,
        }
    }

    /// Create a new reconnect policy with a linear backoff.
    pub fn new_linear(max_attempts: u32, max_delay: u32, delay: u32) -> Self {
        Self::Linear {
            max_attempts,
            max_delay,
            delay,
            jitter: DEFAULT_JITTER_MS,
        }
    }

    /// Create a new reconnect policy with an exponential backoff.
    pub fn new_exponential(
        max_attempts: u32,
        min_delay: u32,
        max_delay: u32,
        multiplicative_factor: u32,
    ) -> Self {
        Self::Exponential {
            max_delay,
            max_attempts,
            min_delay,
            multiplicative_factor,
            jitter: DEFAULT_JITTER_MS,
        }
    }

    /// Set the amount of jitter to add to each reconnection delay.
    ///
    /// Default: 100 ms
    pub fn set_jitter(&mut self, jitter_ms: u32) {
        match self {
            Self::Constant { jitter, .. } => {
                *jitter = jitter_ms;
            }
            Self::Linear { jitter, .. } => {
                *jitter = jitter_ms;
            }
            Self::Exponential { jitter, .. } => {
                *jitter = jitter_ms;
            }
        }
    }
}

/// Percent-decodes a URI component (`%XX` → byte), rendering the result lossily as
/// UTF-8. Malformed escapes are left as-is rather than dropped, so a stray `%`
/// never silently corrupts the surrounding text.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hi) = bytes.get(i + 1).and_then(|b| (*b as char).to_digit(16))
            && let Some(lo) = bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16))
        {
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod parse_tests {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "test code: a panic is how a test reports failure"
    )]
    use super::*;

    fn standalone(uri: &str) -> (String, u16) {
        match Config::from_str(uri).unwrap().server {
            ServerConfig::Standalone { host, port } => (host, port),
            other => panic!("expected Standalone, got {other:?}"),
        }
    }

    #[test]
    fn ipv6_bracketed_address_with_port() {
        assert_eq!(Some(("::1", 6379)), Config::parse_addr("[::1]:6379"));
        assert_eq!(
            Some(("2001:db8::1", 6380)),
            Config::parse_addr("[2001:db8::1]:6380")
        );
    }

    #[test]
    fn ipv6_bracketed_address_without_port() {
        assert_eq!(Some(("::1", DEFAULT_PORT)), Config::parse_addr("[::1]"));
    }

    #[test]
    fn ipv4_address_still_parses() {
        assert_eq!(
            Some(("127.0.0.1", 6379)),
            Config::parse_addr("127.0.0.1:6379")
        );
        assert_eq!(
            Some(("localhost", DEFAULT_PORT)),
            Config::parse_addr("localhost")
        );
    }

    #[test]
    fn ipv6_uri() {
        assert_eq!(("::1".to_owned(), 6379), standalone("redis://[::1]:6379"));
    }

    #[test]
    fn percent_decoded_password() {
        let config = Config::from_str("redis://user:p%40ss@127.0.0.1:6379").unwrap();
        assert_eq!(Some("user".to_owned()), config.username);
        assert_eq!(Some("p@ss".to_owned()), config.password);
    }
}
