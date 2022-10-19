use crate::{Error, Result};
#[cfg(feature = "tls")]
use native_tls::{Certificate, Identity, Protocol, TlsConnector, TlsConnectorBuilder};
use std::{str::FromStr, time::Duration};
use url::Url;

const DEFAULT_PORT: u16 = 6379;
const DEFAULT_DATABASE: usize = 0;

type Uri<'a> = (
    &'a str,
    Option<&'a str>,
    Option<&'a str>,
    Vec<(&'a str, u16)>,
    Vec<&'a str>,
);

#[derive(Clone, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: usize,
    #[cfg(feature = "tls")]
    pub tls_config: Option<TlsConfig>,
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
        let (scheme, username, password, hosts, path_segments) = Self::break_down_uri(uri)?;
        let mut hosts = hosts;
        let mut path_segments = path_segments.into_iter();

        #[cfg(feature = "tls")]
        let (tls_config, is_sentinel) = match scheme {
            "redis" => (None, false),
            "rediss" => (Some(TlsConfig::default()), false),
            "redis+sentinel" => (None, true),
            "rediss+sentinel" => (Some(TlsConfig::default()), true),
            _ => {
                return None;
            }
        };

        #[cfg(not(feature = "tls"))]
        let is_sentinel = match scheme {
            "redis" => false,
            "redis+sentinel" => true,
            _ => {
                return None;
            }
        };

        let server = if is_sentinel {
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

            ServerConfig::Sentinel(SentinelConfig {
                instances,
                service_name,
                ..Default::default()
            })
        } else if hosts.len() > 1 {
            return None;
        } else {
            let (host, port) = hosts.pop()?;
            ServerConfig::Single {
                host: host.to_owned(),
                port,
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

        Some(Config {
            server,
            username: username.map(|u| u.to_owned()),
            password: password.map(|p| p.to_owned()),
            database,
            #[cfg(feature = "tls")]
            tls_config,
        })
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

        let (before_query, _query) = match after_scheme.find('?') {
            Some(index) => match Self::exclusive_split_at(after_scheme, index) {
                (Some(before_query), after_query) => (before_query, after_query),
                _ => {
                    return None;
                }
            },
            None => (after_scheme, None),
        };

        let (authority, path) = match after_scheme.find('/') {
            Some(index) => match Self::exclusive_split_at(before_query, index) {
                (Some(authority), path) => (authority, path),
                _ => {
                    return None;
                }
            },
            None => (after_scheme, None),
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

        Some((scheme, username, password, hosts, path_segments))
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
                ServerConfig::Single { host: _, port: _ } => "rediss://",
                ServerConfig::Sentinel(_) => "rediss+sentinel://",
            }
        } else {
            match &self.server {
                ServerConfig::Single { host: _, port: _ } => "redis://",
                ServerConfig::Sentinel(_) => "redis+sentinel://",
            }
        }.to_owned();

        #[cfg(not(feature = "tls"))]
        let mut s = match &self.server {
            ServerConfig::Single { host: _, port: _ } => "redis://",
            ServerConfig::Sentinel(_) => "redis+sentinel://",
        }.to_owned();

        if let Some(username) = &self.username {
            s.push_str(username);
        }

        if let Some(password) = &self.password {
            s.push(':');
            s.push_str(password);
            s.push('@');
        }

        match &self.server {
            ServerConfig::Single { host, port } => {
                s.push_str(host);
                s.push(':');
                s.push_str(&port.to_string());
            }
            ServerConfig::Sentinel(SentinelConfig {
                instances,
                service_name,
                wait_beetween_failures: _,
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
        }

        if self.database > 0 {
            s.push('/');
            s.push_str(&self.database.to_string());
        }

        s
    }
}

/// Configuration for connecting to a Redis server
#[derive(Clone)]
pub enum ServerConfig {
    /// Connection to a simple server (no master-replica, no cluster)
    Single {
        host: String,
        port: u16,
    },
    Sentinel(SentinelConfig),
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig::Single {
            host: "127.0.0.1".to_owned(),
            port: 6379,
        }
    }
}

/// Configuration for connecting to a Redis via Sentinel
#[derive(Clone)]
pub struct SentinelConfig {
    /// An array of `(host, port)` tuples for each known sentinel instance.
    pub instances: Vec<(String, u16)>,

    /// The service name
    pub service_name: String,

    /// Waiting time after failing before connecting to the next Sentinel instance (default 250ms).
    pub wait_beetween_failures: Duration,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            instances: Default::default(),
            service_name: Default::default(),
            wait_beetween_failures: Duration::from_millis(250),
        }
    }
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

pub trait IntoConfig {
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
            server: ServerConfig::Single {
                host: self.0.into(),
                port: self.1,
            },
            username: None,
            password: None,
            database: 0,
            #[cfg(feature = "tls")]
            tls_config: None,
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
