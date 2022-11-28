use crate::{Message, Result};
use futures::channel::{
    mpsc::{self, TrySendError},
    oneshot,
};
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    num::ParseFloatError,
    str::{FromStr, Utf8Error},
};

/// `Internal Use`
///
/// Gives a reason to retry sending a command to the Redis Server
#[derive(Debug)]
pub enum RetryReason {
    /// Received an ASK error from the Redis Server
    Ask {
        hash_slot: u16,
        address: (String, u16),
    },
    /// Received a MOVED error from the Redis Server
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
    Retry(SmallVec<[RetryReason; 1]>),
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

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Client(format!("{msg}"))
    }
}

impl std::error::Error for Error {}

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    Other,
}

impl RedisErrorKind {
    fn parse_hash_slot_and_address(hash_slot: &str, address: &str) -> Result<(u16, (String, u16))> {
        let hash_slot = hash_slot
            .parse::<u16>()
            .map_err(|_| Error::Client("Cannot parse hash slot".to_owned()))?;
        let (host, port) = address
            .split_once(':')
            .ok_or_else(|| Error::Client("Cannot parse address".to_owned()))?;
        let port = port
            .parse::<u16>()
            .map_err(|_| Error::Client("Cannot parse port".to_owned()))?;
        Ok((hash_slot, (host.to_owned(), port)))
    }
}

impl FromStr for RedisErrorKind {
    type Err = Error;

    fn from_str(str: &str) -> Result<Self> {
        match str {
            "BUSYGROUP" => Ok(Self::BusyGroup),
            "CLUSTERDOWN" => Ok(Self::ClusterDown),
            "CROSSSLOT" => Ok(Self::CrossSlot),
            "ERR" => Ok(Self::Err),
            "INPROG" => Ok(Self::InProg),
            "IOERR" => Ok(Self::IoErr),
            "MASTERDOWN" => Ok(Self::MasterDown),
            "MISCONF" => Ok(Self::MisConf),
            "NOAUTH" => Ok(Self::NoAuth),
            "NOGOODSLAVE" => Ok(Self::NoGoodSlave),
            "NOMASTERLINK" => Ok(Self::NoMasterLink),
            "NOPERM" => Ok(Self::NoPerm),
            "NOPROTO" => Ok(Self::NoProto),
            "NOQUORUM" => Ok(Self::NoQuorum),
            "NOTBUSY" => Ok(Self::NotBusy),
            "OOM" => Ok(Self::OutOfMemory),
            "READONLY" => Ok(Self::Readonly),
            "TRYAGAIN" => Ok(Self::TryAgain),
            "UNKILLABLE" => Ok(Self::UnKillable),
            "UNBLOCKED" => Ok(Self::Unblocked),
            "WRONGPASS" => Ok(Self::WrongPass),
            "WRONGTYPE" => Ok(Self::WrongType),
            _ => {
                let mut iter = str.split_whitespace();
                match (iter.next(), iter.next(), iter.next(), iter.next()) {
                    (Some("ASK"), Some(hash_slot), Some(address), None) => {
                        Self::parse_hash_slot_and_address(hash_slot, address)
                            .map(|(hash_slot, address)| Self::Ask { hash_slot, address })
                    }
                    (Some("MOVED"), Some(hash_slot), Some(address), None) => {
                        Self::parse_hash_slot_and_address(hash_slot, address)
                            .map(|(hash_slot, address)| Self::Moved { hash_slot, address })
                    }
                    _ => Ok(Self::Other),
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
            } => f.write_fmt(format_args!("ASK {} {}:{}", *hash_slot, *host, *port)),
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
            } => f.write_fmt(format_args!("MOVED {} {}:{}", *hash_slot, *host, *port)),
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
            RedisErrorKind::Other => f.write_str(""),
        }
    }
}

/// Error issued by the Redis server
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisError {
    pub kind: RedisErrorKind,
    pub description: String,
}

impl FromStr for RedisError {
    type Err = Error;

    fn from_str(error: &str) -> Result<Self> {
        match error.split_once(' ') {
            Some(("ASK", _)) => Ok(Self {
                kind: RedisErrorKind::from_str(error)?,
                description: "".to_owned(),
            }),
            Some(("MOVED", _)) => Ok(Self {
                kind: RedisErrorKind::from_str(error)?,
                description: "".to_owned(),
            }),
            Some((kind, description)) => {
                let kind = RedisErrorKind::from_str(kind)?;

                let description = if let RedisErrorKind::Other = kind {
                    error.to_owned()
                } else {
                    description.to_owned()
                };

                Ok(Self { kind, description })
            }
            None => Ok(Self {
                kind: RedisErrorKind::Other,
                description: error.to_owned(),
            }),
        }
    }
}

impl Display for RedisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.kind, self.description))
    }
}
