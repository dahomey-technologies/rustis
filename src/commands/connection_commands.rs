use crate::{
    Result,
    client::{PreparedCommand, prepare_command},
    commands::{ModuleInfo, RequestPolicy, ResponsePolicy},
    resp::{Response, cmd, serialize_flag},
};
use serde::{Deserialize, Deserializer, Serialize, de};
use std::collections::HashMap;

/// A group of Redis commands related to connection management
///
/// # See Also
/// [Redis Connection Management Commands](https://redis.io/commands/?group=connection)
pub trait ConnectionCommands<'a>: Sized {
    /// Authenticates the current connection.
    ///
    /// This method supports both the legacy authentication (password only) and
    /// the Redis 6+ ACL authentication (username and password).
    ///
    /// * `username` - The username. Pass `()` to use the default user (legacy behavior).
    /// * `password` - The password.
    ///
    /// # Errors
    /// a Redis error if the password, or username/password pair, is invalid.
    ///
    /// # Warning
    /// Every clone of a [`Client`](crate::client::Client) shares the connection, so
    /// this changes the identity of all clones. Set the credentials in the
    /// [`Config`](crate::client::Config) instead. See
    /// [Connection-scoped commands](crate::client#connection-scoped-commands).
    ///
    /// # Cluster
    /// Authentication is per-connection, and a cluster client holds one connection
    /// per node: authenticating on a single node would leave every other shard
    /// answering as the previous identity.
    ///
    /// # See Also
    /// [<https://redis.io/commands/auth/>](https://redis.io/commands/auth/)
    #[must_use]
    fn auth(
        self,
        username: impl Serialize,
        password: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("AUTH").arg(username).arg(password).cluster_info(
                RequestPolicy::AllNodes,
                ResponsePolicy::AllSucceeded,
                1,
            ),
        )
    }

    /// This command controls the tracking of the keys in the next command executed by the connection,
    /// when tracking is enabled in OPTIN or OPTOUT mode.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-caching/>](https://redis.io/commands/client-caching/)
    #[must_use]
    fn client_caching(self, mode: ClientCachingMode) -> PreparedCommand<'a, Self, Option<()>> {
        prepare_command(self, cmd("CLIENT").arg("CACHING").arg(mode))
    }

    /// Returns the name of the current connection as set by [CLIENT SETNAME].
    ///
    /// # Return
    /// The connection name, or a None if no name is set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-getname/>](https://redis.io/commands/client-getname/)
    #[must_use]
    fn client_getname<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLIENT").arg("GETNAME"))
    }

    /// This command returns the client ID we are redirecting our tracking notifications to.
    ///
    /// # Return
    /// the ID of the client we are redirecting the notifications to.
    /// The command returns -1 if client tracking is not enabled,
    /// or 0 if client tracking is enabled but we are not redirecting the notifications to any client.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-getredir/>](https://redis.io/commands/client-getredir/)
    #[must_use]
    fn client_getredir(self) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("CLIENT").arg("GETREDIR"))
    }

    /// The command returns a helpful text describing the different CLIENT subcommands.
    ///
    /// # Return
    /// An array of strings.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::ConnectionCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// #    let client = Client::connect("127.0.0.1:6379").await?;
    /// let result: Vec<String> = client.client_help().await?;
    /// assert!(result.iter().any(|e| e == "HELP"));
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-help/>](https://redis.io/commands/client-help/)
    #[must_use]
    fn client_help<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("CLIENT").arg("HELP"))
    }

    /// The command just returns the ID of the current connection.
    ///
    /// # Return
    /// The id of the client.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-id/>](https://redis.io/commands/client-id/)
    #[must_use]
    fn client_id(self) -> PreparedCommand<'a, Self, i64> {
        prepare_command(self, cmd("CLIENT").arg("ID"))
    }

    /// The command returns information and statistics about the current client connection
    /// in a mostly human readable format.
    ///
    /// # Return
    /// A ClientInfo struct with additional properties
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-info/>](https://redis.io/commands/client-info/)
    #[must_use]
    fn client_info(self) -> PreparedCommand<'a, Self, ClientInfo> {
        prepare_command(self, cmd("CLIENT").arg("INFO"))
    }

    /// Closes a given clients connection based on a filter list
    ///
    /// # Return
    /// the number of clients killed.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-kill/>](https://redis.io/commands/client-kill/)
    #[must_use]
    fn client_kill(self, options: ClientKillOptions) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("CLIENT").arg("KILL").arg(options))
    }

    /// Returns information and statistics about the client connections server in a mostly human readable format.
    ///
    /// # Return
    /// A Vec of ClientInfo structs with additional properties
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-list/>](https://redis.io/commands/client-list/)
    #[must_use]
    fn client_list(
        self,
        options: ClientListOptions,
    ) -> PreparedCommand<'a, Self, ClientListResult> {
        prepare_command(self, cmd("CLIENT").arg("LIST").arg(options))
    }

    ///  sets the [`client eviction`](https://redis.io/docs/reference/clients/#client-eviction) mode for the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-no-evict/>](https://redis.io/commands/client-no-evict/)
    #[must_use]
    fn client_no_evict(self, no_evict: bool) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLIENT")
                .arg("NO-EVICT")
                .arg(if no_evict { "ON" } else { "OFF" })
                // A cluster client is one connection per node, and eviction is
                // decided by each node for its own clients. Exempting a single
                // node exempts nothing.
                .cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// The command controls whether commands sent by the client will alter the LRU/LFU of the keys they access.
    /// If ON, the client will not change LFU/LRU stats.
    /// If OFF or send TOUCH, client will change LFU/LRU stats just as a normal client.
    ///
    /// # Return
    /// The () type
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::ConnectionCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// #     let client = Client::connect("127.0.0.1:6379").await?;
    /// client.client_no_touch(true).await?;
    /// client.client_no_touch(false).await?;
    /// #     Ok(())
    /// }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/docs/latest/commands/client-no-touch/>](https://redis.io/docs/latest/commands/client-no-touch/)
    #[must_use]
    fn client_no_touch(self, no_touch: bool) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLIENT")
                .arg("NO-TOUCH")
                .arg(if no_touch { "ON" } else { "OFF" })
                // Keys live on every shard, so idle times are only left alone if
                // every node is told to leave them alone.
                .cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// Connections control command able to suspend all the Redis clients
    /// for the specified amount of time (in milliseconds).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-pause/>](https://redis.io/commands/client-pause/)
    #[must_use]
    fn client_pause(self, timeout: u64, mode: ClientPauseMode) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("CLIENT").arg("PAUSE").arg(timeout).arg(mode))
    }

    /// Sometimes it can be useful for clients to completely disable replies from the Redis server.
    ///
    /// # Cluster
    /// A cluster client is one connection per node, so `ON` and `OFF` are sent to
    /// every node: the mode has to be the same everywhere, or a node still answering
    /// would produce a reply nobody accounted for and shift every response after it.
    ///
    /// `SKIP` carries **no routing policy of its own**, because it is only correct on
    /// the nodes reached by the command it silences — one node for a key-routed
    /// command, every shard for a multi-shard one. It is therefore held back and
    /// emitted on exactly those nodes, immediately before that command.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-reply/>](https://redis.io/commands/client-reply/)
    #[must_use]
    fn client_reply(self, mode: ClientReplyMode) -> PreparedCommand<'a, Self, ()> {
        let command = cmd("CLIENT").arg("REPLY").arg(mode);
        prepare_command(
            self,
            match mode {
                ClientReplyMode::Skip => command,
                ClientReplyMode::On | ClientReplyMode::Off => {
                    command.cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1)
                }
            },
        )
    }

    /// Assigns a name to the current connection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-setname/>](https://redis.io/commands/client-setname/)
    #[must_use]
    fn client_setname(self, connection_name: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLIENT")
                .arg("SETNAME")
                .arg(connection_name)
                .cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// Assigns various info attributes to the current connection.
    /// There is no limit to the length of these attributes.
    /// However it is not possible to use spaces, newlines, or other non-printable characters.
    /// Look changes with commands `client_list` or `client_info`.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::{ConnectionCommands, ClientInfoAttribute},
    /// #    resp::cmd,
    /// #    Result,
    /// # };
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// #    let client = Client::connect("127.0.0.1:6379").await?;
    /// client
    ///     .client_setinfo(ClientInfoAttribute::LibName, "rustis")
    ///     .await?;
    /// client
    ///     .client_setinfo(ClientInfoAttribute::LibVer, "0.13.3")
    ///     .await?;
    ///
    /// let attrs: String = client.send(cmd("CLIENT").arg("INFO"), None).await?;
    ///
    /// assert!(attrs.contains("lib-name=rustis lib-ver=0.13.3"));
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/docs/latest/commands/client-setinfo/>](https://redis.io/docs/latest/commands/client-setinfo/)
    #[must_use]
    fn client_setinfo(
        self,
        attr: ClientInfoAttribute,
        info: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLIENT")
                .arg("SETINFO")
                .arg(attr)
                .arg(info)
                .cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// This command enables the tracking feature of the Redis server,
    /// that is used for [`server assisted client side caching`](https://redis.io/topics/client-side-caching).
    ///
    /// # Cluster
    /// Tracking is per-connection and each node only invalidates the keys it
    /// holds, so this is sent to every node: armed on one shard, the keys of every
    /// other shard would be cached and never invalidated.
    ///
    /// Broadcasting it is only sound because no redirection can be expressed — see
    /// [`ClientTrackingOptions`] for why `REDIRECT` is not offered.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-tracking/>](https://redis.io/commands/client-tracking/)
    #[must_use]
    fn client_tracking(
        self,
        status: ClientTrackingStatus,
        options: ClientTrackingOptions,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("CLIENT")
                .arg("TRACKING")
                .arg(status)
                .arg(options)
                .cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// This command enables the tracking feature of the Redis server,
    /// that is used for [`server assisted client side caching`](https://redis.io/topics/client-side-caching).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-tracking/>](https://redis.io/commands/client-tracking/)
    #[must_use]
    fn client_trackinginfo(self) -> PreparedCommand<'a, Self, ClientTrackingInfo> {
        prepare_command(self, cmd("CLIENT").arg("TRACKINGINFO"))
    }

    /// This command can unblock, from a different connection,
    /// a client blocked in a blocking operation,
    /// such as for instance `BRPOP` or `XREAD` or `WAIT`.
    ///
    /// # Return
    /// * `true` - This command can unblock, from a different connection, a client blocked in a blocking operation, such as for instance BRPOP or XREAD or WAIT.
    /// * `false` - if the client wasn't unblocked.
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-unblock/>](https://redis.io/commands/client-unblock/)
    #[must_use]
    fn client_unblock(
        self,
        client_id: i64,
        mode: ClientUnblockMode,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("CLIENT").arg("UNBLOCK").arg(client_id).arg(mode))
    }

    /// Used to resume command processing for all clients that were
    /// paused by [`client_pause`](ConnectionCommands::client_pause).
    ///
    /// # See Also
    /// [<https://redis.io/commands/client-unpause/>](https://redis.io/commands/client-unpause/)
    #[must_use]
    fn client_unpause(self) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("CLIENT").arg("UNPAUSE"))
    }

    /// Returns `message`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/echo/>](https://redis.io/commands/echo/)
    #[must_use]
    fn echo<R: Response>(self, message: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ECHO").arg(message))
    }

    /// Returns PONG if no argument is provided, otherwise return a copy of the argument as a bulk.
    ///
    /// * `message` - if the argument is provided, the command returns a copy of the argument.
    ///   pass `()` to ignore.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ping/>](https://redis.io/commands/ping/)
    #[must_use]
    fn ping<R: Response>(self, message: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("PING").arg(message).cluster_info(
                RequestPolicy::AllShards,
                ResponsePolicy::AllSucceeded,
                1,
            ),
        )
    }

    /// This command performs a full reset of the connection's server-side context,
    /// mimicking the effect of disconnecting and reconnecting again.
    ///
    /// # Cluster
    /// Sent to every node: the client forgets the connection state it was
    /// tracking, so leaving one node holding the old state would put the two out
    /// of step in the direction that produces a stale replay on the next
    /// reconnection.
    ///
    /// # See Also
    /// [<https://redis.io/commands/reset/>](https://redis.io/commands/reset/)
    #[must_use]
    fn reset(self) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("RESET").cluster_info(RequestPolicy::AllNodes, ResponsePolicy::AllSucceeded, 1),
        )
    }

    /// Select the Redis logical database having the specified zero-based numeric index.
    ///
    /// # Warning
    /// Every clone of a [`Client`](crate::client::Client) shares the connection, so
    /// this moves the commands of all clones to `index`. Use
    /// [`Config::database`](crate::client::Config::database) instead. See
    /// [Connection-scoped commands](crate::client#connection-scoped-commands).
    ///
    /// # Cluster
    /// A cluster has a single database, and the server answers any non-zero index
    /// with *"SELECT is not allowed in cluster mode"*. No routing policy is declared
    /// for that reason: there is nothing to broadcast, and the server's own refusal
    /// is clearer than one this client would invent.
    ///
    /// # See Also
    /// [<https://redis.io/commands/select/>](https://redis.io/commands/select/)
    #[must_use]
    fn select(self, index: usize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("SELECT").arg(index))
    }
}

/// Client caching mode for the [`client_caching`](ConnectionCommands::client_caching) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ClientCachingMode {
    Yes,
    No,
}

/// Client info results for the [`client_info`](ConnectionCommands::client_info)
/// & [`client_list`](ConnectionCommands::client_list) commands.
#[derive(Debug)]
#[non_exhaustive]
pub struct ClientInfo {
    /// a unique 64-bit client ID
    pub id: i64,

    /// address/port of the client
    pub addr: String,

    /// address/port of local address client connected to (bind address)
    pub laddr: String,

    /// file descriptor corresponding to the socket
    pub fd: u32,

    /// the name set by the client with [`client_setname`](ConnectionCommands::client_setname)
    pub name: String,

    /// total duration of the connection in seconds
    pub age: u32,

    /// idle time of the connection in seconds
    pub idle: u32,

    /// client flags (see [`client-list`](https://redis.io/commands/client-list/))
    pub flags: String,

    /// current database ID
    pub db: usize,

    /// number of channel subscriptions
    pub sub: usize,

    /// number of pattern matching subscriptions
    pub psub: usize,

    /// number of shard channel subscriptions. Added in Redis 7.0.3
    pub ssub: usize,

    /// number of commands in a MULTI/EXEC context
    pub multi: usize,

    /// query buffer length (0 means no query pending)
    pub qbuf: usize,

    /// free space of the query buffer (0 means the buffer is full)
    pub qbuf_free: usize,

    /// incomplete arguments for the next command (already extracted from query buffer)
    pub argv_mem: usize,

    /// memory is used up by buffered multi commands. Added in Redis 7.0
    pub multi_mem: usize,

    /// output buffer length
    pub obl: usize,

    /// output list length (replies are queued in this list when the buffer is full)
    pub oll: usize,

    /// output buffer memory usage
    pub omem: usize,

    ///  total memory consumed by this client in its various buffers
    pub tot_mem: usize,

    /// file descriptor events (r or w)
    pub events: String,

    /// last command played
    pub cmd: String,

    /// the authenticated username of the client
    pub user: String,

    /// client id of current client tracking redirection
    pub redir: i64,

    /// client RESP protocol version
    pub resp: i32,

    /// additional arguments that may be added in future versions of Redis
    pub additional_arguments: HashMap<String, String>,
}

impl ClientInfo {
    pub fn from_line(line: &str) -> Result<ClientInfo> {
        // Each line is composed of a succession of property=value fields separated by a space character.
        let mut values: HashMap<String, String> = line
            .trim_end()
            .split(' ')
            .map(|kvp| {
                let mut iter = kvp.split('=');
                match (iter.next(), iter.next()) {
                    (Some(key), None) => (key.to_owned(), "".to_owned()),
                    (Some(key), Some(value)) => (key.to_owned(), value.to_owned()),
                    _ => ("".to_owned(), "".to_owned()),
                }
            })
            .collect();

        Ok(ClientInfo {
            id: values
                .remove("id")
                .map(|id| id.parse::<i64>().unwrap_or_default())
                .unwrap_or_default(),
            addr: values.remove("addr").unwrap_or_default(),
            laddr: values.remove("laddr").unwrap_or_default(),
            fd: values
                .remove("fd")
                .map(|id| id.parse::<u32>().unwrap_or_default())
                .unwrap_or_default(),
            name: values.remove("name").unwrap_or_default(),
            age: values
                .remove("age")
                .map(|id| id.parse::<u32>().unwrap_or_default())
                .unwrap_or_default(),
            idle: values
                .remove("idle")
                .map(|id| id.parse::<u32>().unwrap_or_default())
                .unwrap_or_default(),
            flags: values.remove("flags").unwrap_or_default(),
            db: values
                .remove("db")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            sub: values
                .remove("sub")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            psub: values
                .remove("psub")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            ssub: values
                .remove("ssub")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            multi: values
                .remove("multi")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            qbuf: values
                .remove("qbuf")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            qbuf_free: values
                .remove("qbuf-free")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            argv_mem: values
                .remove("argv-mem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            multi_mem: values
                .remove("multi-mem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            obl: values
                .remove("obl")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            oll: values
                .remove("oll")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            omem: values
                .remove("omem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            tot_mem: values
                .remove("tot-mem")
                .map(|id| id.parse::<usize>().unwrap_or_default())
                .unwrap_or_default(),
            events: values.remove("events").unwrap_or_default(),
            cmd: values.remove("cmd").unwrap_or_default(),
            user: values.remove("user").unwrap_or_default(),
            redir: values
                .remove("redir")
                .map(|id| id.parse::<i64>().unwrap_or_default())
                .unwrap_or_default(),
            resp: values
                .remove("resp")
                .map(|id| id.parse::<i32>().unwrap_or_default())
                .unwrap_or_default(),
            additional_arguments: values,
        })
    }
}

impl<'de> Deserialize<'de> for ClientInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let line = <&str>::deserialize(deserializer)?;
        ClientInfo::from_line(line).map_err(de::Error::custom)
    }
}

/// Client type options for the [`client_list`](ConnectionCommands::client_list) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ClientType {
    Normal,
    Master,
    Replica,
    PubSub,
}

/// Options for the [client_list](ConnectionCommands::client_list) command.
#[derive(Default, Serialize)]
pub struct ClientListOptions {
    #[serde(rename = "TYPE", skip_serializing_if = "Option::is_none")]
    client_type: Option<ClientType>,
    #[serde(rename = "ID", skip_serializing_if = "Vec::is_empty")]
    client_ids: Vec<i64>,
}

impl ClientListOptions {
    #[must_use]
    pub fn client_type(mut self, client_type: ClientType) -> Self {
        self.client_type = Some(client_type);
        self
    }

    pub fn client_ids(mut self, client_ids: impl IntoIterator<Item = i64>) -> Self {
        self.client_ids.extend(client_ids);
        self
    }

    pub fn client_id(mut self, client_id: i64) -> Self {
        self.client_ids.push(client_id);
        self
    }
}

/// Result for the [`client_list`](ConnectionCommands::client_list) command.
#[derive(Debug)]
#[non_exhaustive]
pub struct ClientListResult {
    pub client_infos: Vec<ClientInfo>,
}

impl<'de> Deserialize<'de> for ClientListResult {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let lines = <&str>::deserialize(deserializer)?;
        // The reply is newline-terminated, so the split yields a trailing empty
        // line that carries no client.
        let client_infos: Result<Vec<ClientInfo>> = lines
            .split('\n')
            .filter(|line| !line.trim().is_empty())
            .map(ClientInfo::from_line)
            .collect();

        Ok(Self {
            client_infos: client_infos.map_err(de::Error::custom)?,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
enum YesNo {
    Yes,
    No,
}

/// Options for the [`client-kill`](ConnectionCommands::client-kill) command.
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct ClientKillOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    #[serde(rename = "TYPE", skip_serializing_if = "Option::is_none")]
    client_type: Option<ClientType>,
    #[serde(rename = "USER", skip_serializing_if = "Option::is_none")]
    username: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    laddr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skipme: Option<YesNo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxage: Option<u64>,
}

impl<'a> ClientKillOptions<'a> {
    #[must_use]
    pub fn id(mut self, client_id: i64) -> Self {
        self.id = Some(client_id);
        self
    }

    #[must_use]
    pub fn client_type(mut self, client_type: ClientType) -> Self {
        self.client_type = Some(client_type);
        self
    }

    #[must_use]
    pub fn user(mut self, username: &'a str) -> Self {
        self.username = Some(username);
        self
    }

    /// Address in the format of `ip:port`
    ///
    /// The ip:port should match a line returned by the
    /// [`client_list`](ConnectionCommands::client_list) command (addr field).
    #[must_use]
    pub fn addr(mut self, ip: &'a str, port: u16) -> Self {
        self.addr = Some(format!("{ip}:{port}"));
        self
    }

    /// Kill all clients connected to specified local (bind) address.
    #[must_use]
    pub fn laddr(mut self, ip: &'a str, port: u16) -> Self {
        self.laddr = Some(format!("{ip}:{port}"));
        self
    }

    /// By default this option is set to yes, that is, the client calling the command will not get killed,
    /// however setting this option to no will have the effect of also killing the client calling the command.
    #[must_use]
    pub fn skip_me(mut self, skip_me: bool) -> Self {
        self.skipme = Some(if skip_me { YesNo::Yes } else { YesNo::No });
        self
    }

    ///  Closes all the connections that are older than the specified age, in seconds.
    #[must_use]
    pub fn max_age(mut self, seconds: u64) -> Self {
        self.maxage = Some(seconds);
        self
    }
}

/// Mode options for the [`client_pause`](ConnectionCommands::client_pause) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ClientPauseMode {
    /// Clients are only blocked if they attempt to execute a write command.
    Write,
    /// This is the default mode. All client commands are blocked.
    #[default]
    All,
}

/// Mode options for the [`client_reply`](ConnectionCommands::client_reply) command.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ClientReplyMode {
    On,
    Off,
    Skip,
}

/// Status options for the [`client_tracking`](ConnectionCommands::client_tracking) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ClientTrackingStatus {
    On,
    Off,
}

/// Options for the [`client_tracking`](ConnectionCommands::client_tracking) command.
///
/// # `REDIRECT` is not offered
///
/// `CLIENT TRACKING … REDIRECT <client-id>` sends invalidations to another
/// connection instead of this one. It exists for **RESP2**, which cannot deliver an
/// invalidation on a connection that is also answering commands: the target
/// subscribes to `__redis__:invalidate` and reads them as pub/sub messages.
///
/// rustis always negotiates **RESP3** (`HELLO 3`, at every connect and reconnect),
/// where invalidations arrive as push frames on the very connection that enabled
/// tracking — which is what
/// [`create_client_tracking_invalidation_stream`](crate::client::Client::create_client_tracking_invalidation_stream)
/// and [`Cache`](crate::cache::Cache) consume. A redirection therefore has no
/// destination worth using, and setting one would silently starve both: they stay
/// alive, report no error, and simply never fire again.
///
/// Two further reasons it is not merely useless but unexpressible here: a client id
/// is a **per-node counter**, so on a cluster client the same number designates a
/// different connection on each node; and the id names a connection this client does
/// not own, whose lifetime it cannot track.
#[derive(Clone, Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct ClientTrackingOptions {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    prefix: Vec<String>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    bcast: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    optin: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    optout: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    noloop: bool,
}

impl ClientTrackingOptions {
    /// enable tracking in broadcasting mode.
    pub fn broadcasting(mut self) -> Self {
        self.bcast = true;
        self
    }

    /// for broadcasting, register a given key prefix, so that notifications
    /// will be provided only for keys starting with this string.
    ///
    /// This option can be given multiple times to register multiple prefixes.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix.push(prefix.into());
        self
    }

    /// when broadcasting is NOT active, normally don't track keys in read only commands,
    /// unless they are called immediately after a `CLIENT CACHING yes` command.
    pub fn optin(mut self) -> Self {
        self.optin = true;
        self
    }

    /// when broadcasting is NOT active, normally track keys in read only commands,
    /// unless they are called immediately after a `CLIENT CACHING no` command.
    pub fn optout(mut self) -> Self {
        self.optout = true;
        self
    }

    /// don't send notifications about keys modified by this connection itself.
    pub fn noloop(mut self) -> Self {
        self.noloop = true;
        self
    }
}

/// Result for the [`client_trackinginfo`](ConnectionCommands::client_trackinginfo) command.
#[derive(Deserialize)]
#[non_exhaustive]
pub struct ClientTrackingInfo {
    /// A list of tracking flags used by the connection.
    pub flags: Vec<String>,

    /// The client ID used for notifications redirection, or -1 when none.
    pub redirect: i64,

    /// A list of key prefixes for which notifications are sent to the client.
    pub prefixes: Vec<String>,
}

/// Mode options for the [`client_unblock`](ConnectionCommands::client_unblock) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ClientUnblockMode {
    /// By default the client is unblocked as if the timeout of the command was reached,
    #[default]
    Timeout,
    /// the behavior is to unblock the client returning as error the fact that the client was force-unblocked.
    Error,
}

/// Options for the `HELLO` command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) struct HelloOptions<'a> {
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    protover: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<(&'a str, &'a str)>,

    #[serde(skip_serializing_if = "Option::is_none")]
    setname: Option<&'a str>,
}

impl<'a> HelloOptions<'a> {
    #[must_use]
    pub(crate) fn new(protover: u32) -> Self {
        Self {
            protover: Some(protover),
            ..Default::default()
        }
    }

    #[must_use]
    pub(crate) fn auth(mut self, username: &'a str, password: &'a str) -> Self {
        self.auth = Some((username, password));
        self
    }

    #[must_use]
    pub(crate) fn set_name(mut self, client_name: &'a str) -> Self {
        self.setname = Some(client_name);
        self
    }
}

/// Result for the `HELLO` command
///
/// Every field the reply carries is kept, rather than only the one the handshake
/// reads: the struct is the shape of the reply, and serde needs a field to consume
/// each entry of the map.
#[derive(Deserialize)]
// The handshake reads `version` and nothing else, so the other fields are dead in
// the library. They are not dead in the suite, which asserts on them, hence the
// `not(test)`: an unconditional `expect` would itself go unfulfilled under `cfg(test)`.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the handshake reads `version`; the rest mirror the HELLO reply"
    )
)]
pub(crate) struct HelloResult {
    pub server: String,
    pub version: String,
    pub proto: usize,
    pub id: i64,
    pub mode: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub modules: Vec<ModuleInfo>,
}

// Info options for the [`client_setinfo`](ConnectionCommands::client_setinfo) command.
#[derive(Serialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
#[non_exhaustive]
pub enum ClientInfoAttribute {
    LibName,
    LibVer,
}
