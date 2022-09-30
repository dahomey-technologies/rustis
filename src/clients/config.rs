use crate::{Error, Result};
use url::Url;

const DEFAULT_PORT: u16 = 6379;
const DEFAULT_DATABASE: usize = 0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub database: usize,
}

impl Config {
    /// Build a config from an URI a standard address format `host`:`port`
    pub fn from_str(str: &str) -> Result<Config> {
        if let Ok(uri) = url::Url::parse(str) {
            return Self::from_uri(uri);
        }

        if let Some((host, port)) = Self::parse_addr(str) {
            return Ok(Self {
                host,
                port,
                username: None,
                password: None,
                database: 0,
            });
        }

        Err(Error::Config(format!("Cannot parse config from {str}")))
    }

    /// Build a config from an URI in the format `redis://[[username]:password@]host[:port]/[database]`
    pub fn from_uri(uri: Url) -> Result<Config> {
        if uri.scheme() == "redis" {
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
                    });
                }
            }
        }

        Err(Error::Config(format!("Cannot parse config from {uri}")))
    }

    /// A string in the standard formart `host`:`port`
    pub fn to_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
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
        let mut s = String::from("redis://");

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
