use crate::Message;
use futures::channel::{
    mpsc::{self, TrySendError},
    oneshot,
};
use std::{num::ParseFloatError, str::Utf8Error};

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Redis(String),
    Parse(String),
    Send(String),
    SendError(String),
    Canceled(oneshot::Canceled),
    Internal(String),
    Network(String),
    Aborted,
    Config(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => f.write_fmt(format_args!("IO: {}", e)),
            Error::Redis(e) => f.write_fmt(format_args!("Redis: {}", e)),
            Error::Parse(e) => f.write_fmt(format_args!("Parse: {}", e)),
            Error::Send(e) => f.write_fmt(format_args!("Send: {}", e)),
            Error::SendError(e) => f.write_fmt(format_args!("SendError: {}", e)),
            Error::Canceled(e) => f.write_fmt(format_args!("oneshot::Canceled: {}", e)),
            Error::Internal(e) => f.write_fmt(format_args!("Internal error: {}", e)),
            Error::Network(e) => f.write_fmt(format_args!("Network error: {}", e)),
            Error::Aborted => f.write_fmt(format_args!("Transaction aborted")),
            Error::Config(e) => f.write_fmt(format_args!("Config error: {}", e)),
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
        Error::Send(e.to_string())
    }
}

impl From<oneshot::Canceled> for Error {
    fn from(e: oneshot::Canceled) -> Self {
        Error::Canceled(e)
    }
}

impl From<mpsc::SendError> for Error {
    fn from(e: mpsc::SendError) -> Self {
        Error::SendError(e.to_string())
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Parse(e.to_string())
    }
}

impl From<ParseFloatError> for Error {
    fn from(e: ParseFloatError) -> Self {
        Error::Parse(e.to_string())
    }
}

impl From<tokio_native_tls::native_tls::Error> for Error {
    fn from(e: tokio_native_tls::native_tls::Error) -> Self {
        Error::Network(e.to_string())
    }
}
