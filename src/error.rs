use crate::Message;
use futures::channel::{
    mpsc::{self, TrySendError},
    oneshot,
};
use std::{num::ParseFloatError, str::Utf8Error};

/// All error kinds
#[derive(Debug)]
pub enum Error {
    /// Raised if an error occurs within the driver
    Client(String),
    /// Raised if an error occurs in the [`Config`](crate::Config) parsing
    Config(String),
    /// A transaction has been aborted
    Aborted,
    /// Error returned by the Redis sercer
    Redis(String),
    /// IO error when connecting the Redis server
    IO(std::io::Error),
    #[cfg(feature = "tls")]
    /// Raised by the TLS library
    Tls(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Client(e) => f.write_fmt(format_args!("Client error: {}", e)),
            Error::Config(e) => f.write_fmt(format_args!("Config error: {}", e)),
            Error::Aborted => f.write_fmt(format_args!("Transaction aborted")),
            Error::Redis(e) => f.write_fmt(format_args!("Redis error: {}", e)),
            Error::IO(e) => f.write_fmt(format_args!("IO erro: {}", e)),
            #[cfg(feature = "tls")]
            Error::Tls(e) => f.write_fmt(format_args!("Tls error: {}", e)),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<TrySendError<Message>> for Error {
    fn from(e: TrySendError<Message>) -> Self {
        Error::Client(e.to_string())
    }
}

impl From<oneshot::Canceled> for Error {
    fn from(e: oneshot::Canceled) -> Self {
        Error::Client(e.to_string())
    }
}

impl From<mpsc::SendError> for Error {
    fn from(e: mpsc::SendError) -> Self {
        Error::Client(e.to_string())
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Client(e.to_string())
    }
}

impl From<ParseFloatError> for Error {
    fn from(e: ParseFloatError) -> Self {
        Error::Client(e.to_string())
    }
}

#[cfg(feature = "tls")]
impl From<native_tls::Error> for Error {
    fn from(e: native_tls::Error) -> Self {
        Error::Tls(e.to_string())
    }
}
