use crate::{Error, Result};
#[cfg(feature = "tls")]
use native_tls::{Certificate, Identity, Protocol, TlsConnector, TlsConnectorBuilder};
use std::{collections::HashMap, str::FromStr, time::Duration};
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
#[derive(Debug, Clone)]
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
    #[cfg_attr(docsrs, doc(cfg(feature = "tls")))]
    #[cfg(feature = "tls")]
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
    /// Reconnection configuration (Constant, Linear or Exponential)
    pub reconnection: ReconnectionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Default::default(),
            username: Default::default(),
            password: Default::default(),
            database: Default::default(),
            #[cfg(feature = "tls")]
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
            Err(Error::Config(format!("Cannot parse config from {str}")))
        }
    }
}

impl Config {
    /// Build a config from an URI in the format `redis[s]://[[username]:password@]host[:port]/[database]`
    pub fn from_uri(uri: Url) -> Result<Config> {
        Self::from_str(uri.as_str())
    }

    /// Parse address in the standard formart `host`:`port`
    fn parse_addr(str: &str) -> Option<(&str, u16)> {
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

        #[cfg(feature = "tls")]
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

        #[cfg(not(feature = "tls"))]
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
                    if let Some(millis) = query.remove("wait_between_failures") {
                        if let Ok(millis) = millis.parse::<u64>() {
                            sentinel_config.wait_between_failures = Duration::from_millis(millis);
                        }
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
            username: username.map(|u| u.to_owned()),
            password: password.map(|p| p.to_owned()),
            database,
            #[cfg(feature = "tls")]
            tls_config,
            ..Default::default()
        };

        if let Some(ref mut query) = query {
            if let Some(millis) = query.remove("connect_timeout") {
                if let Ok(millis) = millis.parse::<u64>() {
                    config.connect_timeout = Duration::from_millis(millis);
                }
            }

            if let Some(millis) = query.remove("command_timeout") {
                if let Ok(millis) = millis.parse::<u64>() {
                    config.command_timeout = Duration::from_millis(millis);
                }
            }

            if let Some(auto_resubscribe) = query.remove("auto_resubscribe") {
                if let Ok(auto_resubscribe) = auto_resubscribe.parse::<bool>() {
                    config.auto_resubscribe = auto_resubscribe;
                }
            }

            if let Some(auto_remonitor) = query.remove("auto_remonitor") {
                if let Ok(auto_remonitor) = auto_remonitor.parse::<bool>() {
                    config.auto_remonitor = auto_remonitor;
                }
            }

            if let Some(connection_name) = query.remove("connection_name") {
                config.connection_name = connection_name;
            }

            if let Some(keep_alive) = query.remove("keep_alive") {
                if let Ok(keep_alive) = keep_alive.parse::<u64>() {
                    config.keep_alive = Some(Duration::from_millis(keep_alive));
                }
            }

            if let Some(no_delay) = query.remove("no_delay") {
                if let Ok(no_delay) = no_delay.parse::<bool>() {
                    config.no_delay = no_delay;
                }
            }

            if let Some(retry_on_error) = query.remove("retry_on_error") {
                if let Ok(retry_on_error) = retry_on_error.parse::<bool>() {
                    config.retry_on_error = retry_on_error;
                }
            }
        }

        Some(config)
    }

    /// break down an uri in a tuple (scheme, username, password, hosts, path_segments)
    fn break_down_uri(uri: &str) -> Option<Uri> {
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

impl ToString for Config {
    fn to_string(&self) -> String {
        #[cfg(feature = "tls")]
        let mut s = if self.tls_config.is_some() {
            match &self.server {
                ServerConfig::Standalone { host: _, port: _ } => "rediss://",
                ServerConfig::Sentinel(_) => "rediss+sentinel://",
                ServerConfig::Cluster(_) => "rediss+cluster://",
            }
        } else {
            match &self.server {
                ServerConfig::Standalone { host: _, port: _ } => "redis://",
                ServerConfig::Sentinel(_) => "redis+sentinel://",
                ServerConfig::Cluster(_) => "redis+cluster://",
            }
        }
        .to_owned();

        #[cfg(not(feature = "tls"))]
        let mut s = match &self.server {
            ServerConfig::Standalone { host: _, port: _ } => "redis://",
            ServerConfig::Sentinel(_) => "redis+sentinel://",
            ServerConfig::Cluster(_) => "redis+cluster://",
        }
        .to_owned();

        if let Some(username) = &self.username {
            s.push_str(username);
        }

        if let Some(password) = &self.password {
            s.push(':');
            s.push_str(password);
            s.push('@');
        }

        match &self.server {
            ServerConfig::Standalone { host, port } => {
                s.push_str(host);
                if *port != DEFAULT_PORT {
                    s.push(':');
                    s.push_str(&port.to_string());
                }
            }
            ServerConfig::Sentinel(SentinelConfig {
                instances,
                service_name,
                wait_between_failures: _,
                password: _,
                username: _,
            }) => {
                s.push_str(
                    &instances
                        .iter()
                        .map(|(host, port)| format!("{host}:{port}"))
                        .collect::<Vec<String>>()
                        .join(","),
                );
                s.push('/');
                s.push_str(service_name);
            }
            ServerConfig::Cluster(ClusterConfig { nodes }) => {
                s.push_str(
                    &nodes
                        .iter()
                        .map(|(host, port)| format!("{host}:{port}"))
                        .collect::<Vec<String>>()
                        .join(","),
                );
            }
        }

        if self.database > 0 {
            s.push('/');
            s.push_str(&self.database.to_string());
        }

        // query

        let mut query_separator = false;

        let connect_timeout = self.connect_timeout.as_millis() as u64;
        if connect_timeout != DEFAULT_CONNECT_TIMEOUT {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("connect_timeout={connect_timeout}"));
        }

        let command_timeout = self.command_timeout.as_millis() as u64;
        if command_timeout != DEFAULT_COMMAND_TIMEOUT {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("command_timeout={command_timeout}"));
        }

        if self.auto_resubscribe != DEFAULT_AUTO_RESUBSCRTBE {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("auto_resubscribe={}", self.auto_resubscribe));
        }

        if self.auto_remonitor != DEFAULT_AUTO_REMONITOR {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("auto_remonitor={}", self.auto_remonitor));
        }

        if !self.connection_name.is_empty() {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("connection_name={}", self.connection_name));
        }

        if let Some(keep_alive) = self.keep_alive {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("keep_alive={}", keep_alive.as_millis()));
        }

        if self.no_delay != DEFAULT_NO_DELAY {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("no_delay={}", self.no_delay));
        }

        if self.retry_on_error != DEFAULT_RETRY_ON_ERROR {
            if !query_separator {
                query_separator = true;
                s.push('?');
            } else {
                s.push('&');
            }
            s.push_str(&format!("retry_on_error={}", self.retry_on_error));
        }

        if let ServerConfig::Sentinel(SentinelConfig {
            instances: _,
            service_name: _,
            wait_between_failures: wait_beetween_failures,
            password,
            username,
        }) = &self.server
        {
            let wait_between_failures = wait_beetween_failures.as_millis() as u64;
            if wait_between_failures != DEFAULT_WAIT_BETWEEN_FAILURES {
                if !query_separator {
                    query_separator = true;
                    s.push('?');
                } else {
                    s.push('&');
                }
                s.push_str(&format!("wait_between_failures={wait_between_failures}"));
            }
            if let Some(username) = username {
                if !query_separator {
                    query_separator = true;
                    s.push('?');
                } else {
                    s.push('&');
                }
                s.push_str("sentinel_username=");
                s.push_str(username);
            }
            if let Some(password) = password {
                if !query_separator {
                    s.push('?');
                } else {
                    s.push('&');
                }
                s.push_str("sentinel_password=");
                s.push_str(password);
            }
        }

        s
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
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    /// An array of `(host, port)` tuples for each known sentinel instance.
    pub instances: Vec<(String, u16)>,

    /// The service name
    pub service_name: String,

    /// Waiting time after failing before connecting to the next Sentinel instance (default 250ms).
    pub wait_between_failures: Duration,

    /// Sentinel username
    pub username: Option<String>,

    /// Sentinel password
    pub password: Option<String>,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            instances: Default::default(),
            service_name: Default::default(),
            wait_between_failures: Duration::from_millis(DEFAULT_WAIT_BETWEEN_FAILURES),
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
/// See [TlsConnectorBuilder](https://docs.rs/tokio-native-tls/0.3.0/tokio_native_tls/native_tls/struct.TlsConnectorBuilder.html) documentation
#[cfg(feature = "tls")]
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

#[cfg(feature = "tls")]
impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            identity: None,
            root_certificates: None,
            min_protocol_version: Some(Protocol::Tlsv10),
            max_protocol_version: None,
            disable_built_in_roots: false,
            danger_accept_invalid_certs: false,
            danger_accept_invalid_hostnames: false,
            use_sni: true,
        }
    }
}

#[cfg(feature = "tls")]
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

#[cfg(feature = "tls")]
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
    /// see https://en.wikipedia.org/wiki/Exponential_backoff
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
            Self::Constant { ref mut jitter, .. } => {
                *jitter = jitter_ms;
            }
            Self::Linear { ref mut jitter, .. } => {
                *jitter = jitter_ms;
            }
            Self::Exponential { ref mut jitter, .. } => {
                *jitter = jitter_ms;
            }
        }
    }
}
