use crate::{Error, Result};
use native_tls::{Certificate, Identity, Protocol, TlsConnector, TlsConnectorBuilder};
use url::Url;

const DEFAULT_PORT: u16 = 6379;
const DEFAULT_DATABASE: usize = 0;

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: usize,
    pub tls_config: Option<TlsConfig>,
}

impl Config {
    /// Build a config from an URI a standard address format `host`:`port`
    pub fn from_str(str: &str) -> Result<Config> {
        if let Ok(uri) = url::Url::parse(str) {
            Self::from_uri(uri)
        } else if let Some(addr) = Self::parse_addr(str) {
            addr.into_config()
        } else {
            Err(Error::Config(format!("Cannot parse config from {str}")))
        }
    }

    /// Build a config from an URI in the format `redis://[[username]:password@]host[:port]/[database]`
    pub fn from_uri(uri: Url) -> Result<Config> {
        let scheme = uri.scheme();
        if scheme == "redis" || scheme == "rediss" {
            if let Some(host) = uri.host_str() {
                let host = host.to_owned();
                let port = uri.port().unwrap_or(DEFAULT_PORT);
                let password = uri.password();
                let password = password.map(|p| p.to_owned());
                let username = uri.username();
                let username = match username {
                    "" => None,
                    u => Some(u.to_owned()),
                };
                let database = match uri.path() {
                    "" => DEFAULT_DATABASE,
                    path => path[1..]
                        .parse::<usize>()
                        .map_err(|_| Error::Config(format!("Cannot parse config from {uri}")))?,
                };

                // username without password is not accepted
                if username.is_none() || password.is_some() {
                    return Ok(Config {
                        host,
                        port,
                        username,
                        password,
                        database,
                        tls_config: if scheme == "rediss" {
                            Some(TlsConfig::default())
                        } else {
                            None
                        },
                    });
                }
            }
        }

        Err(Error::Config(format!("Cannot parse config from {uri}")))
    }

    /// Parse address in the standard formart `host`:`port`
    fn parse_addr(str: &str) -> Option<(String, u16)> {
        let mut iter = str.split(':');

        match (iter.next(), iter.next(), iter.next()) {
            (Some(host), Some(port), None) => {
                if let Ok(port) = port.parse::<u16>() {
                    Some((host.to_owned(), port))
                } else {
                    None
                }
            }
            (Some(host), None, None) => Some((host.to_owned(), DEFAULT_PORT)),
            _ => None,
        }
    }
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let mut s = String::from(if self.tls_config.is_some() {
            "rediss://"
        } else {
            "redis://"
        });

        if let Some(username) = &self.username {
            s.push_str(&username);
        }

        if let Some(password) = &self.password {
            s.push(':');
            s.push_str(password);
            s.push('@');
        }

        s.push_str(&self.host);
        s.push(':');
        s.push_str(&self.port.to_string());

        if self.database > 0 {
            s.push('/');
            s.push_str(&self.database.to_string());
        }

        s
    }
}

/// Config for TLS.
///
/// See [TlsConnectorBuilder](https://docs.rs/tokio-native-tls/0.3.0/tokio_native_tls/native_tls/struct.TlsConnectorBuilder.html) documentation
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

    pub fn into_tls_connector_builder(&self) ->TlsConnectorBuilder {
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
        return Ok(self);
    }
}

impl<T: Into<String>> IntoConfig for (T, u16) {
    fn into_config(self) -> Result<Config> {
        Ok(Config {
            host: self.0.into(),
            port: self.1,
            username: None,
            password: None,
            database: 0,
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
