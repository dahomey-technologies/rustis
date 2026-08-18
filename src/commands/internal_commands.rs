use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{HelloOptions, HelloResult},
    resp::{Response, cmd},
};

/// The commands that drive the connection, sent by the client and not by the caller.
///
/// A [`Client`](crate::client::Client) is clonable, and one connection carries the
/// commands of every clone. Each of these commands reconfigures that shared
/// connection, or contradicts a decision the client has already made:
///
/// - `HELLO` sets the protocol version. The handshake fixes it at RESP3, which every
///   deserializer requires.
/// - `READONLY` and `READWRITE` set the read mode that
///   [`ClusterConfig::read_preference`](crate::client::ClusterConfig::read_preference)
///   depends on.
/// - `ASKING` is correct only immediately before the command it redirects.
/// - `CLUSTER SLOTS` is deprecated since Redis 7.0. Callers use `cluster_shards`.
///
/// The client sends each command at the point where it is correct. The generic
/// command API can still send them, which stays the caller's responsibility.
pub(crate) trait InternalCommands<'a>: Sized {
    /// When a cluster client receives an -ASK redirect,
    /// the ASKING command is sent to the target node followed by the command which was redirected.
    ///
    /// # See Also
    /// [<https://redis.io/commands/asking/>](https://redis.io/commands/asking/)
    #[must_use]
    fn asking(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("ASKING"))
    }

    /// This command returns details about which cluster slots map to which Redis instances.
    ///
    /// Deprecated by Redis 7.0 in favour of `CLUSTER SHARDS`, and used only where
    /// that one does not exist: topology discovery against a pre-7.0 server.
    ///
    /// # See Also
    /// [<https://redis.io/commands/cluster-slots/>](https://redis.io/commands/cluster-slots/)
    fn cluster_slots<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLUSTER").arg("SLOTS"))
    }

    /// Switch to a different protocol,
    /// optionally authenticating and setting the connection's name,
    /// or provide a contextual client report.
    ///
    /// # See Also
    /// [<https://redis.io/commands/hello/>](https://redis.io/commands/hello/)
    #[must_use]
    fn hello(self, options: HelloOptions) -> PreparedCommand<'a, Self, HelloResult> {
        prepare_command(self, cmd("HELLO").arg(options))
    }

    /// Enables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readonly/>](https://redis.io/commands/readonly/)
    #[must_use]
    fn readonly(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("READONLY"))
    }

    /// Disables read queries for a connection to a Redis Cluster replica node.
    ///
    /// # See Also
    /// [<https://redis.io/commands/readwrite/>](https://redis.io/commands/readwrite/)
    #[must_use]
    fn readwrite(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("READWRITE"))
    }
}
