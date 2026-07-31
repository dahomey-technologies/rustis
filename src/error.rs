use crate::Result;
use atoi::atoi;
use futures_channel::mpsc;
use smallvec::SmallVec;
use std::{
    fmt::{Display, Formatter},
    num::{ParseFloatError, ParseIntError},
    str::Utf8Error,
    string::FromUtf8Error,
    sync::Arc,
};
use thiserror::Error;

/// `Internal Use`
///
/// Gives a reason to retry sending a command to the Redis Server
#[doc(hidden)]
#[derive(Debug, Clone)]
#[non_exhaustive]
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
    /// Received a transient cluster error (`TRYAGAIN`, `CLUSTERDOWN`) from the
    /// Redis Server: the command was not executed and the cluster spec asks the
    /// client to replay it after a short delay.
    TryAgain {
        /// How long to wait before the command is sent again.
        delay: std::time::Duration,
        /// Whether the local topology is suspect and has to be reloaded before
        /// the replay: true for `CLUSTERDOWN`, which follows a topology change,
        /// false for `TRYAGAIN`, which only reports a slot in migration.
        refresh_topology: bool,
    },
}

/// Errors issued by the client
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum ClientError {
    /// Raised when an invalid RESP tag is encountered
    #[error("protocol: invalid tag")]
    InvalidTag,
    /// Raised when an expected array result is not received for MGET command
    #[error("protocol: expected array result for MGET")]
    ExpectedArrayForMGet,
    /// Raised when cannot parse integer from the RESP buffer
    #[error("protocol: cannot parse integer")]
    CannotParseInteger,
    /// Raised when cannot parse double from the RESP buffer
    #[error("protocol: cannot parse double")]
    CannotParseDouble,
    /// Raised when cannot parse bulk string from the RESP buffer
    #[error("protocol: cannot parse bulk string")]
    CannotParseBulkString,
    /// Raised when cannot parse bulk error from the RESP buffer
    #[error("protocol: cannot parse bulk error")]
    CannotParseBulkError,
    /// Raised when cannot parse verbartim string from the RESP buffer
    #[error("protocol: cannot parse verbartim string")]
    CannotParseVerbatimString,
    /// Raised when cannot parse nil from the RESP buffer
    #[error("protocol: cannot parse nil")]
    CannotParseNil,
    /// Raised when cannot parse boolean from the RESP buffer
    #[error("protocol: cannot parse boolean")]
    CannotParseBoolean,
    /// Raised when cannot parse char from the RESP buffer
    #[error("protocol: cannot parse char")]
    CannotParseChar,
    /// Raised when cannot parse str from the RESP buffer
    #[error("protocol: cannot parse str")]
    CannotParseStr,
    /// Raised when cannot parse string from the RESP buffer
    #[error("protocol: cannot parse string")]
    CannotParseString,
    /// Raised when cannot parse sequence from the RESP buffer
    #[error("protocol: cannot parse sequence")]
    CannotParseSequence,
    /// Raised when cannot parse map from the RESP buffer
    #[error("protocol: cannot parse map")]
    CannotParseMap,
    /// Raised when cannot parse struct from the RESP buffer
    #[error("protocol: cannot parse struct")]
    CannotParseStruct,
    /// Raised when cannot parse bytes from the RESP buffer
    #[error("protocol: cannot parse bytes")]
    CannotParseBytes,
    /// Raised when cannot parse enum from the RESP buffer
    #[error("protocol: cannot parse enum")]
    CannotParseEnum,
    /// Raised when verbatim string is too short
    #[error("protocol: verbatim string too short")]
    VerbatimStringTooShort,
    /// Raised when an unknown RESP tag is encountered
    #[error("protocol: unknown RESP tag {0}")]
    UnknownRespTag(char),
    /// Raised when disconnected from the server
    #[error("disconnected from server")]
    DisconnectedFromServer,
    /// Raised when an invalid channel to send messages to the network handler is used
    #[error("invalid channel to send messages to the network handler")]
    InvalidChannel,
    /// Raised when client is already subscribed to the given channel/pattern
    #[error("client is already subscribed to the given channel/pattern")]
    AlreadySubscribed,
    /// Raised when the server sends a subscription confirmation that does not
    /// match the pending subscription request (out-of-order or spurious ack)
    #[error("unexpected subscription confirmation from server")]
    UnexpectedSubscriptionConfirmation,
    /// Raised when serde serialization error occurs
    #[error("Serde deserialization error: {0}")]
    SerdeDeserialize(String),
    /// Raised when serde serialization error occurs
    #[error("Serde serialization error: {0}")]
    SerdeSerialize(String),
    /// Raised when a command groups its arguments by a step of zero, which names
    /// no group at all. The step is a caller-supplied width on the builder's
    /// `*_with_count_and_step` methods, so the command carries the error instead of
    /// dividing its argument count by zero.
    #[error("command args: a group step of zero is not a valid grouping")]
    InvalidArgumentGroupStep,
    /// Raised when a command has been retried up to `Config::max_command_attempts`
    /// without succeeding, so it is failed instead of retried indefinitely.
    #[error("command failed after reaching the maximum number of attempts")]
    MaxCommandAttemptsReached,
    /// Raised when the send queue has reached `Config::backpressure.max_queued_bytes`,
    /// so an incoming command is shed instead of growing the queue further.
    ///
    /// This means the connection is down and the queue of commands waiting for it
    /// is full. It is distinct from
    /// [`DisconnectedByPeer`](crate::Error::DisconnectedByPeer), which means the
    /// command was dropped because it opted out of retries, and from
    /// [`MaxCommandAttemptsReached`](Self::MaxCommandAttemptsReached), which means
    /// the command was retried and kept failing. Only a *new* command is refused:
    /// one already queued, or replayed after a reconnection or a redirection,
    /// never is.
    #[error("send queue is full")]
    SendQueueFull,
    #[error("a client-side cache key must serialize to exactly one argument")]
    InvalidCacheKey,
    /// Raised when an unexpected error occurs
    #[error("Unexpected error")]
    Unexpected,
    /// Raised when cannot parse hash slot
    #[error("cannot parse hash slot")]
    CannotParseHashSlot,
    /// Raised when cannot parse address
    #[error("cannot parse address")]
    CannotParseAddress,
    /// Raised when cannot parse port
    #[error("cannot parse port")]
    CannotParsePort,
    /// Raised when cannot parse RequestPolicy
    #[error("Cannot parse RequestPolicy")]
    CannotParseRequestPolicy,
    /// Raised when cannot parse ResponsePolicy
    #[error("Cannot parse ResponsePolicy")]
    CannotParseResponsePolicy,
    /// Raised if an error occurs in the [`Config`](crate::client::Config) parsing
    #[error("Cannot parse config")]
    ConfigParseError,
    /// Raised when a connection URI carries a query parameter that is unknown or
    /// whose value cannot be parsed. The message names the offending parameter.
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    /// Raised at connection time when a [`Config`](crate::client::Config) knob
    /// holds a value that would disable behavior rather than tune it — a zero
    /// buffer capacity, a zero loop bound. The message names the offending knob.
    #[error("Invalid config: {0}")]
    InvalidConfig(&'static str),
    /// Raised when the client's own routing state stops agreeing with itself — a
    /// node index or a pending-request index that no longer addresses anything.
    /// Unreachable by construction: every such index is produced by scanning the
    /// very collection it is then used on. It is reported rather than asserted
    /// because these lookups happen on the network task, where a panic would take
    /// down every in-flight command and the reconnection loop with them.
    #[error("inconsistent internal routing state")]
    InconsistentRoutingState,
    /// Raised if an error occurs in the [`ClusterConfig`](crate::client::ClusterConfig)
    #[error("Cluster misconfiguration")]
    ClusterConfig,
    /// Raised when EXEC is called without MULTI
    #[error("EXEC called without MULTI")]
    ExecCalledWithoutMulti,
    /// Raised when a transaction mixes keys belonging to different hash slots,
    /// which Redis Cluster cannot execute atomically
    #[error("CROSSSLOT Keys in request don't hash to the same slot")]
    CrossSlot,
    /// Raised when a command is not supported in cluster mode
    #[error("Command not supported in cluster mode")]
    CommandNotSupportedInCluster,
    /// Raised when an unexpected message is received
    #[error("Unexpected message received")]
    UnexpectedMessageReceived,
    /// Raised when keys hash slots do not match
    #[error("Keys hash slots do not match")]
    MismatchedKeySlots,
    /// Raised when cannot parse Redis server version
    #[error("Cannot parse Redis server version")]
    CannotParseRedisServerVersion,
    /// Raised when a RESP frame nests collections deeper than the parser allows,
    /// guarding against a crafted reply driving the parser into a stack overflow.
    #[error("protocol: maximum nesting depth exceeded")]
    MaxNestingDepthExceeded,
    /// Raised when a bulk string / bulk error / verbatim string declares a
    /// length beyond the parser's configured ceiling, before the payload is
    /// trusted — stops a crafted header from driving unbounded buffering.
    #[error("protocol: bulk length exceeds the maximum allowed")]
    BulkLengthTooLarge,
    /// Raised when a collection (array / set / push / map) declares a cardinality
    /// beyond the parser's configured ceiling.
    #[error("protocol: collection length exceeds the maximum allowed")]
    CollectionLengthTooLarge,
}

/// All error kinds
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum Error {
    /// Raised if an error occurs within the driver
    #[error("client error: {0}")]
    Client(#[from] ClientError),
    /// Raised if a required cache key is in the wrong type
    #[error("cache wrong key type")]
    CacheWrongKeyType,
    /// A transaction has been aborted
    #[error("transaction aborted")]
    Aborted,
    /// Raised if an error occurs when contacting Sentinel instances
    #[error("sentinel error: {0}")]
    Sentinel(String),
    /// Error returned by the Redis server
    #[error("redis server error: {0}")]
    Redis(#[from] RedisError),
    /// IO error when connecting the Redis server
    #[error("io error: {0}")]
    IO(Arc<std::io::Error>),
    /// Raised by the TLS library
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    #[error("tls error: {0}")]
    Tls(Arc<native_tls::Error>),
    /// Raised by the TLS library
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(feature = "rustls")]
    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),
    /// Invalid Dns name (rustls)
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
    #[cfg(feature = "rustls")]
    #[error("invalid dns name: {0}")]
    InvalidDnsName(Arc<rustls::pki_types::InvalidDnsNameError>),
    /// The I/O operation’s timeout expired
    #[error("The I/O operation’s timeout expired")]
    Timeout,
    /// Internal error to trigger retry sending the command
    #[doc(hidden)]
    #[error("Retry")]
    Retry(SmallVec<[RetryReason; 1]>),
    /// Raised when end of stream is reached
    #[error("End of stream reached")]
    EOF,
    /// Raised when a tokio join error occurs
    #[cfg(feature = "tokio-runtime")]
    #[error("tokio join error: {0}")]
    TokioJoin(Arc<tokio::task::JoinError>),
    /// Raised when oneshot channel is canceled
    #[error("oneshot channel canceled")]
    OneshotCanceled(#[from] tokio::sync::oneshot::error::RecvError),
    /// Raised when mpsc send error occurs
    #[error("mpsc send error: {0}")]
    MpscSend(#[from] mpsc::SendError),
    /// Raised when UTF-8 error occurs
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] Utf8Error),
    /// Raised when FromUtf8 error occurs
    #[error("FromUtf8 error: {0}")]
    FromUtf8(#[from] FromUtf8Error),
    /// Raised when parse float error occurs
    #[error("Parse float error: {0}")]
    ParseFloat(#[from] ParseFloatError),
    /// Raised when parse int error occurs
    #[error("Parse int error: {0}")]
    ParseInt(#[from] ParseIntError),
    /// Raised when tokio broadcast send error occurs
    #[error("Tokio broadcast send error: {0}")]
    TokioBroadcastSend(Arc<tokio::sync::broadcast::error::SendError<()>>),
    /// Disconnected by peer
    #[error("Disconnected by peer")]
    DisconnectedByPeer,
}

impl From<tokio::sync::broadcast::error::SendError<()>> for Error {
    fn from(value: tokio::sync::broadcast::error::SendError<()>) -> Self {
        Error::TokioBroadcastSend(Arc::new(value))
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(Arc::new(value))
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
#[cfg(feature = "native-tls")]
impl From<native_tls::Error> for Error {
    fn from(value: native_tls::Error) -> Self {
        Error::Tls(Arc::new(value))
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "rustls")))]
#[cfg(feature = "rustls")]
impl From<rustls::pki_types::InvalidDnsNameError> for Error {
    fn from(value: rustls::pki_types::InvalidDnsNameError) -> Self {
        Error::InvalidDnsName(Arc::new(value))
    }
}

#[cfg(feature = "tokio-runtime")]
impl From<tokio::task::JoinError> for Error {
    fn from(value: tokio::task::JoinError) -> Self {
        Error::TokioJoin(Arc::new(value))
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Client(ClientError::SerdeDeserialize(msg.to_string()))
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::Client(ClientError::SerdeSerialize(msg.to_string()))
    }
}

/// Redis server error kind
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
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
    NoScript,
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
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`index` is a separator position found inside `address`, so stepping \
                  past it stays an offset into the slice."
    )]
    fn parse_hash_slot_and_address(
        hash_slot: &[u8],
        address: &[u8],
    ) -> Result<(u16, (String, u16))> {
        let hash_slot = atoi(hash_slot).ok_or(Error::Client(ClientError::CannotParseHashSlot))?;
        // Split at the last colon: IPv6 hosts contain colons, and Redis emits
        // bare `host:port` with no brackets, so only the rightmost colon
        // reliably separates the port.
        let index = address
            .iter()
            .rposition(|b| *b == b':')
            .ok_or(Error::Client(ClientError::CannotParseAddress))?;
        let (host, port) = (&address[..index], &address[index + 1..]);
        let port = atoi(port).ok_or(Error::Client(ClientError::CannotParsePort))?;
        Ok((hash_slot, (String::from_utf8_lossy(host).to_string(), port)))
    }
}

impl<'a> TryFrom<&'a [u8]> for RedisErrorKind {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> std::result::Result<Self, Self::Error> {
        match value {
            b"BUSYGROUP" => Ok(Self::BusyGroup),
            b"CLUSTERDOWN" => Ok(Self::ClusterDown),
            b"CROSSSLOT" => Ok(Self::CrossSlot),
            b"ERR" => Ok(Self::Err),
            b"INPROG" => Ok(Self::InProg),
            b"IOERR" => Ok(Self::IoErr),
            b"MASTERDOWN" => Ok(Self::MasterDown),
            b"MISCONF" => Ok(Self::MisConf),
            b"NOAUTH" => Ok(Self::NoAuth),
            b"NOGOODSLAVE" => Ok(Self::NoGoodSlave),
            b"NOMASTERLINK" => Ok(Self::NoMasterLink),
            b"NOPERM" => Ok(Self::NoPerm),
            b"NOPROTO" => Ok(Self::NoProto),
            b"NOQUORUM" => Ok(Self::NoQuorum),
            b"NOTBUSY" => Ok(Self::NotBusy),
            b"NOSCRIPT" => Ok(Self::NoScript),
            b"OOM" => Ok(Self::OutOfMemory),
            b"READONLY" => Ok(Self::Readonly),
            b"TRYAGAIN" => Ok(Self::TryAgain),
            b"UNKILLABLE" => Ok(Self::UnKillable),
            b"UNBLOCKED" => Ok(Self::Unblocked),
            b"WRONGPASS" => Ok(Self::WrongPass),
            b"WRONGTYPE" => Ok(Self::WrongType),
            _ => {
                let mut iter = value.split(u8::is_ascii_whitespace);
                match (iter.next(), iter.next(), iter.next(), iter.next()) {
                    (Some(b"ASK"), Some(hash_slot), Some(address), None) => {
                        Self::parse_hash_slot_and_address(hash_slot, address)
                            .map(|(hash_slot, address)| Self::Ask { hash_slot, address })
                    }
                    (Some(b"MOVED"), Some(hash_slot), Some(address), None) => {
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
            RedisErrorKind::NoScript => f.write_str("NOSCRIPT"),
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
pub struct RedisError {
    pub kind: RedisErrorKind,
    pub description: String,
}

impl<'a> TryFrom<&'a [u8]> for RedisError {
    type Error = Error;

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`i` is a separator position found inside `error`, so stepping past \
                  it stays an offset into the slice."
    )]
    fn try_from(error: &'a [u8]) -> std::result::Result<Self, Self::Error> {
        match error
            .iter()
            .position(|b| *b == b' ')
            .map(|i| (&error[..i], &error[i + 1..]))
        {
            Some((b"ASK", _)) => Ok(Self {
                kind: RedisErrorKind::try_from(error)?,
                description: "".to_owned(),
            }),
            Some((b"MOVED", _)) => Ok(Self {
                kind: RedisErrorKind::try_from(error)?,
                description: "".to_owned(),
            }),
            Some((kind, description)) => {
                let kind = RedisErrorKind::try_from(kind)?;

                let description = if let RedisErrorKind::Other = kind {
                    error
                } else {
                    description
                };

                Ok(Self {
                    kind,
                    description: String::from_utf8_lossy(description).to_string(),
                })
            }
            None => Ok(Self {
                kind: RedisErrorKind::Other,
                description: String::from_utf8_lossy(error).to_string(),
            }),
        }
    }
}

impl Display for RedisError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.kind, self.description))
    }
}
