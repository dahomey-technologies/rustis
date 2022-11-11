use crate::Message;
use futures::channel::{
    mpsc::{self, TrySendError},
    oneshot,
};
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    num::ParseFloatError,
    str::Utf8Error,
};

#[derive(Debug)]
pub enum RetryReason {
    Ask {
        hash_slot: u16,
        address: (String, u16),
    },
    Moved {
        hash_slot: u16,
        address: (String, u16),
    },
}

/// All error kinds
#[derive(Debug)]
pub enum Error {
    /// Raised if an error occurs within the driver
    Client(String),
    /// Raised if an error occurs in the [`Config`](crate::Config) parsing
    Config(String),
    /// A transaction has been aborted
    Aborted,
    /// Raised if an error occurs when contacting Sentinel instances
    Sentinel(String),
    /// Error returned by the Redis sercer
    Redis(RedisError),
    /// IO error when connecting the Redis server
    IO(std::io::Error),
    #[cfg(feature = "tls")]
    /// Raised by the TLS library
    Tls(String),
    /// Internal error to trigger retry sending the command
    Retry(SmallVec<[RetryReason;5]>)
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Client(e) => f.write_fmt(format_args!("Client error: {}", e)),
            Error::Config(e) => f.write_fmt(format_args!("Config error: {}", e)),
            Error::Aborted => f.write_fmt(format_args!("Transaction aborted")),
            Error::Sentinel(e) => f.write_fmt(format_args!("Sentinel error: {}", e)),
            Error::Redis(e) => f.write_fmt(format_args!("Redis error: {}", e)),
            Error::IO(e) => f.write_fmt(format_args!("IO erro: {}", e)),
            #[cfg(feature = "tls")]
            Error::Tls(e) => f.write_fmt(format_args!("Tls error: {}", e)),
            Error::Retry(r) => f.write_fmt(format_args!("Retry: {:?}", r)),
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

/// Redis server error kind
#[derive(Debug, Clone)]
pub enum RedisErrorKind {
    Ask {
        hash_slot: u16,
        address: (String, u16),
    },
    BusyGroup,
    ClusterDown,
    CrossSlot,
    Err,
    InProg,
    IoErr,
    MasterDown,
    MisConf,
    Moved {
        hash_slot: u16,
        address: (String, u16),
    },
    NoAuth,
    NoGoodSlave,
    NoMasterLink,
    NoPerm,
    NoProto,
    NoQuorum,
    NotBusy,
    OutOfMemory,
    Readonly,
    TryAgain,
    UnKillable,
    Unblocked,
    WrongPass,
    WrongType,
    Other(String),
}

impl From<&str> for RedisErrorKind {
    fn from(str: &str) -> Self {
        match str {
            "BUSYGROUP" => Self::BusyGroup,
            "CLUSTERDOWN" => Self::ClusterDown,
            "CROSSSLOT" => Self::CrossSlot,
            "ERR" => Self::Err,
            "INPROG" => Self::InProg,
            "IOERR" => Self::IoErr,
            "MASTERDOWN" => Self::MasterDown,
            "MISCONF" => Self::MisConf,
            "NOAUTH" => Self::NoAuth,
            "NOGOODSLAVE" => Self::NoGoodSlave,
            "NOMASTERLINK" => Self::NoMasterLink,
            "NOPERM" => Self::NoPerm,
            "NOPROTO" => Self::NoProto,
            "NOQUORUM" => Self::NoQuorum,
            "NOTBUSY" => Self::NotBusy,
            "OOM" => Self::OutOfMemory,
            "READONLY" => Self::Readonly,
            "TRYAGAIN" => Self::TryAgain,
            "UNKILLABLE" => Self::UnKillable,
            "UNBLOCKED" => Self::Unblocked,
            "WRONGPASS" => Self::WrongPass,
            "WRONGTYPE" => Self::WrongType,
            _ => {
                let mut iter = str.split_whitespace();
                match (iter.next(), iter.next(), iter.next(), iter.next()) {
                    (Some("ASK"), Some(hash_slot), Some(address), None) => {
                        let hash_slot = hash_slot.parse::<u16>().unwrap();
                        let (host, port) = address.split_once(':').unwrap();
                        let port = port.parse::<u16>().unwrap();
                        Self::Ask {
                            hash_slot,
                            address: (host.to_owned(), port),
                        }
                    }
                    (Some("MOVED"), Some(hash_slot), Some(address), None) => {
                        let hash_slot = hash_slot.parse::<u16>().unwrap();
                        let (host, port) = address.split_once(':').unwrap();
                        let port = port.parse::<u16>().unwrap();
                        Self::Moved {
                            hash_slot,
                            address: (host.to_owned(), port),
                        }
                    }
                    _ => Self::Other(str.to_owned()),
                }
            }
        }
    }
}

impl Display for RedisErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisErrorKind::Ask {
                hash_slot,
                address: (host, port),
            } => f.write_fmt(format_args!(
                "ASK {} {}:{}",
                *hash_slot, *host, *port
            )),
            RedisErrorKind::BusyGroup => f.write_str("BUSYGROUP"),
            RedisErrorKind::ClusterDown => f.write_str("CLUSTERDOWN"),
            RedisErrorKind::CrossSlot => f.write_str("CROSSSLOT"),
            RedisErrorKind::Err => f.write_str("ERR"),
            RedisErrorKind::InProg => f.write_str("INPROG"),
            RedisErrorKind::IoErr => f.write_str("IOERR"),
            RedisErrorKind::MasterDown => f.write_str("MASTERDOWN"),
            RedisErrorKind::MisConf => f.write_str("MISCONF"),
            RedisErrorKind::Moved {
                hash_slot,
                address: (host, port),
            } => f.write_fmt(format_args!(
                "MOVED {} {}:{}",
                *hash_slot, *host, *port
            )),
            RedisErrorKind::NoAuth => f.write_str("NOAUTH"),
            RedisErrorKind::NoGoodSlave => f.write_str("NOGOODSLAVE"),
            RedisErrorKind::NoMasterLink => f.write_str("NOMASTERLINK"),
            RedisErrorKind::NoPerm => f.write_str("NOPERM"),
            RedisErrorKind::NoProto => f.write_str("NOPROTO"),
            RedisErrorKind::NoQuorum => f.write_str("NOQUORUM"),
            RedisErrorKind::NotBusy => f.write_str("NOTBUSY"),
            RedisErrorKind::OutOfMemory => f.write_str("OOM"),
            RedisErrorKind::Readonly => f.write_str("READONLY"),
            RedisErrorKind::TryAgain => f.write_str("TRYAGAIN"),
            RedisErrorKind::UnKillable => f.write_str("UNKILLABLE"),
            RedisErrorKind::Unblocked => f.write_str("UNBLOCKED"),
            RedisErrorKind::WrongPass => f.write_str("WRONGPASS"),
            RedisErrorKind::WrongType => f.write_str("WRONGTYPE"),
            RedisErrorKind::Other(e) => f.write_str(e),
        }
    }
}

/// Error issued by the Redis server
#[derive(Debug, Clone)]
pub struct RedisError {
    pub kind: RedisErrorKind,
    pub description: String,
}

impl From<&str> for RedisError {
    fn from(error: &str) -> Self {
        match error.split_once(' ') {
            Some(("ASK", _)) => {
                Self {
                    kind: error.into(),
                    description: "".to_owned(),
                }
            },
            Some(("MOVED", _)) => {
                Self {
                    kind: error.into(),
                    description: "".to_owned(),
                }
            },
            Some((kind, description)) => {
                Self {
                    kind: kind.into(),
                    description: description.to_owned(),
                }
            }
            None => {
                    Self {
                        kind: error.into(),
                        description: "".to_owned(),
                    }
                }
        }
    }
}

impl Display for RedisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.kind, self.description))
    }
}
