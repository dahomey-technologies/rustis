use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, CommandArgs, Response, cmd},
};
use serde::{Deserialize, de::DeserializeOwned};
use std::collections::HashMap;

/// A group of Redis commands related to [`Streams`](https://redis.io/docs/data-types/streams/)
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=stream)
/// [Streams tutorial](https://redis.io/docs/data-types/streams-tutorial/)
pub trait StreamCommands<'a>: Sized {
    /// The XACK command removes one or multiple messages
    /// from the Pending Entries List (PEL) of a stream consumer group
    ///
    /// # Return
    /// The command returns the number of messages successfully acknowledged.
    /// Certain message IDs may no longer be part of the PEL (for example because they have already been acknowledged),
    /// and XACK will not count them as successfully acknowledged.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xack/>](https://redis.io/commands/xack/)
    fn xack(
        self,
        key: impl Args,
        group: impl Args,
        ids: impl Args,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XACK").arg(key).arg(group).arg(ids))
    }

    /// Appends the specified stream entry to the stream at the specified key.
    ///
    /// # Return
    /// the ID of the added entry.
    ///
    /// The ID is the one auto-generated if * is passed as ID argument,
    /// otherwise the command just returns the same ID specified by the user during insertion.
    ///
    /// The command returns a Null reply when used with create_stream=false and the key doesn't exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xadd/>](https://redis.io/commands/xadd/)
    fn xadd<R: Response>(
        self,
        key: impl Args,
        stream_id: impl Args,
        items: impl Args,
        options: XAddOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XADD").arg(key).arg(options).arg(stream_id).arg(items),
        )
    }

    /// This command transfers ownership of pending stream entries that match the specified criteria.
    ///
    /// # Return
    /// An instance of StreamAutoClaimResult
    ///
    /// # See Also
    /// [<https://redis.io/commands/xautoclaim/>](https://redis.io/commands/xautoclaim/)
    fn xautoclaim<R: Response + DeserializeOwned>(
        self,
        key: impl Args,
        group: impl Args,
        consumer: impl Args,
        min_idle_time: u64,
        start: impl Args,
        options: XAutoClaimOptions,
    ) -> PreparedCommand<'a, Self, XAutoClaimResult<R>> {
        prepare_command(
            self,
            cmd("XAUTOCLAIM")
                .arg(key)
                .arg(group)
                .arg(consumer)
                .arg(min_idle_time)
                .arg(start)
                .arg(options),
        )
    }

    /// In the context of a stream consumer group, this command changes the ownership of a pending message,
    /// so that the new owner is the consumer specified as the command argument.
    ///
    /// # Return
    /// The ID of the added entry.
    ///
    /// The ID is the one auto-generated if * is passed as ID argument,
    /// otherwise the command just returns the same ID specified by the user during insertion.
    ///
    /// The command returns a Null reply when used with create_stream=false and the key doesn't exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xclaim/>](https://redis.io/commands/xclaim/)
    fn xclaim<R: Response>(
        self,
        key: impl Args,
        group: impl Args,
        consumer: impl Args,
        min_idle_time: u64,
        ids: impl Args,
        options: XClaimOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XCLAIM")
                .arg(key)
                .arg(group)
                .arg(consumer)
                .arg(min_idle_time)
                .arg(ids)
                .arg(options),
        )
    }

    /// Removes the specified entries from a stream, and returns the number of entries deleted.
    ///
    /// # Return
    /// The number of entries actually deleted.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xdel/>](https://redis.io/commands/xdel/)
    fn xdel(self, key: impl Args, ids: impl Args) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XDEL").arg(key).arg(ids))
    }

    /// This command creates a new consumer group uniquely identified by `groupname` for the stream stored at `key`.
    ///
    /// # Return
    /// * `true` success
    /// * `false`failure
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-create/>](https://redis.io/commands/xgroup-create/)
    fn xgroup_create(
        self,
        key: impl Args,
        groupname: impl Args,
        id: impl Args,
        options: XGroupCreateOptions,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("CREATE")
                .arg(key)
                .arg(groupname)
                .arg(id)
                .arg(options),
        )
    }

    /// Create a consumer named `consumername` in the consumer group `groupname``
    /// of the stream that's stored at `key.
    ///
    /// # Return
    /// * `true` success
    /// * `false`failure
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-createconsumer/>](https://redis.io/commands/xgroup-createconsumer/)
    fn xgroup_createconsumer(
        self,
        key: impl Args,
        groupname: impl Args,
        consumername: impl Args,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("CREATECONSUMER")
                .arg(key)
                .arg(groupname)
                .arg(consumername),
        )
    }

    /// The XGROUP DELCONSUMER command deletes a consumer from the consumer group.
    ///
    /// # Return
    /// The number of pending messages that the consumer had before it was deleted
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-delconsumer/>](https://redis.io/commands/xgroup-delconsumer/)
    fn xgroup_delconsumer(
        self,
        key: impl Args,
        groupname: impl Args,
        consumername: impl Args,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("DELCONSUMER")
                .arg(key)
                .arg(groupname)
                .arg(consumername),
        )
    }

    /// The XGROUP DESTROY command completely destroys a consumer group.
    ///
    /// # Return
    /// * `true` success
    /// * `false`failure
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-destroy/>](https://redis.io/commands/xgroup-destroy/)
    fn xgroup_destroy(
        self,
        key: impl Args,
        groupname: impl Args,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("XGROUP").arg("DESTROY").arg(key).arg(groupname))
    }

    /// The command returns a helpful text describing the different XGROUP subcommands.
    ///
    /// # Return
    /// An array of strings.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::StreamCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// # #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// # async fn main() -> Result<()> {
    /// #    let client = Client::connect("127.0.0.1:6379").await?;
    /// let result: Vec<String> = client.xgroup_help().await?;
    /// assert!(result.iter().any(|e| e == "HELP"));
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-help/>](https://redis.io/commands/xgroup-help/)
    #[must_use]
    fn xgroup_help(self) -> PreparedCommand<'a, Self, Vec<String>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("XGROUP").arg("HELP"))
    }

    /// Set the last delivered ID for a consumer group.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-setid/>](https://redis.io/commands/xgroup-setid/)
    fn xgroup_setid(
        self,
        key: impl Args,
        groupname: impl Args,
        id: impl Args,
        entries_read: Option<usize>,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("SETID")
                .arg(key)
                .arg(groupname)
                .arg(id)
                .arg(entries_read.map(|e| ("ENTRIESREAD", e))),
        )
    }

    /// This command returns the list of consumers that belong to the `groupname` consumer group of the stream stored at `key`.
    ///
    /// # Return
    /// A collection of XConsumerInfo.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-consumers/>](https://redis.io/commands/xinfo-consumers/)
    fn xinfo_consumers(
        self,
        key: impl Args,
        groupname: impl Args,
    ) -> PreparedCommand<'a, Self, Vec<XConsumerInfo>> {
        prepare_command(self, cmd("XINFO").arg("CONSUMERS").arg(key).arg(groupname))
    }

    /// This command returns the list of consumers that belong
    /// to the `groupname` consumer group of the stream stored at `key`.
    ///
    /// # Return
    /// A collection of XGroupInfo.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-groups/>](https://redis.io/commands/xinfo-groups/)
    fn xinfo_groups(self, key: impl Args) -> PreparedCommand<'a, Self, Vec<XGroupInfo>> {
        prepare_command(self, cmd("XINFO").arg("GROUPS").arg(key))
    }

    /// The command returns a helpful text describing the different XINFO subcommands.
    ///
    /// # Return
    /// An array of strings.
    ///
    /// # Example
    /// ```
    /// # use rustis::{
    /// #    client::Client,
    /// #    commands::StreamCommands,
    /// #    Result,
    /// # };
    /// #
    /// # #[cfg_attr(feature = "tokio-runtime", tokio::main)]
    /// # #[cfg_attr(feature = "async-std-runtime", async_std::main)]
    /// # async fn main() -> Result<()> {
    /// #    let client = Client::connect("127.0.0.1:6379").await?;
    /// let result: Vec<String> = client.xinfo_help().await?;
    /// assert!(result.iter().any(|e| e == "HELP"));
    /// #   Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-help/>](https://redis.io/commands/xinfo-help/)
    #[must_use]
    fn xinfo_help(self) -> PreparedCommand<'a, Self, Vec<String>>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("XINFO").arg("HELP"))
    }

    /// This command returns information about the stream stored at `key`.
    ///
    /// # Return
    /// A collection of XGroupInfo.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-stream/>](https://redis.io/commands/xinfo-stream/)
    fn xinfo_stream(
        self,
        key: impl Args,
        options: XInfoStreamOptions,
    ) -> PreparedCommand<'a, Self, XStreamInfo> {
        prepare_command(self, cmd("XINFO").arg("STREAM").arg(key).arg(options))
    }

    /// Returns the number of entries inside a stream.
    ///
    /// # Return
    /// The number of entries of the stream at `key`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrange/>](https://redis.io/commands/xrange/)
    fn xlen(self, key: impl Args) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XLEN").arg(key))
    }

    /// The XPENDING command is the interface to inspect the list of pending messages.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xpending/>](https://redis.io/commands/xpending/)
    fn xpending(
        self,
        key: impl Args,
        group: impl Args,
    ) -> PreparedCommand<'a, Self, XPendingResult> {
        prepare_command(self, cmd("XPENDING").arg(key).arg(group))
    }

    /// The XPENDING command is the interface to inspect the list of pending messages.
    ///
    ///
    /// # Return
    /// A collection of (`XPendingMessageResult`)[XPendingMessageResult]
    ///
    /// # See Also
    /// [<https://redis.io/commands/xpending/>](https://redis.io/commands/xpending/)
    fn xpending_with_options<R: Response>(
        self,
        key: impl Args,
        group: impl Args,
        options: XPendingOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("XPENDING").arg(key).arg(group).arg(options))
    }

    /// The command returns the stream entries matching a given range of IDs.
    ///
    /// # Return
    /// A collection of (`StreamEntry`)[StreamEntry]
    ///
    /// The command returns the entries with IDs matching the specified range.
    /// The returned entries are complete, that means that the ID and all the fields they are composed are returned.
    /// Moreover, the entries are returned with their fields and values in the exact same order as XADD added them.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrange/>](https://redis.io/commands/xrange/)
    fn xrange<R: Response>(
        self,
        key: impl Args,
        start: impl Args,
        end: impl Args,
        count: Option<usize>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XRANGE")
                .arg(key)
                .arg(start)
                .arg(end)
                .arg(count.map(|c| ("COUNT", c))),
        )
    }

    /// Read data from one or multiple streams,
    /// only returning entries with an ID greater than the last received ID reported by the caller.
    ///
    /// # Return
    /// A collection of (`StreamEntry`)[StreamEntry]
    ///
    /// # See Also
    /// [<https://redis.io/commands/xread/>](https://redis.io/commands/xread/)
    fn xread<R: Response>(
        self,
        options: XReadOptions,
        keys: impl Args,
        ids: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XREAD").arg(options).arg("STREAMS").arg(keys).arg(ids),
        )
    }

    /// The XREADGROUP command is a special version of the [`xread`](StreamCommands::xread)
    /// command with support for consumer groups.
    ///
    /// # Return
    /// A collection of (`StreamEntry`)[StreamEntry]
    ///
    /// # See Also
    /// [<https://redis.io/commands/xreadgroup/>](https://redis.io/commands/xreadgroup/)
    fn xreadgroup<R: Response>(
        self,
        group: impl Args,
        consumer: impl Args,
        options: XReadGroupOptions,
        keys: impl Args,
        ids: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XREADGROUP")
                .arg("GROUP")
                .arg(group)
                .arg(consumer)
                .arg(options)
                .arg("STREAMS")
                .arg(keys)
                .arg(ids),
        )
    }

    /// This command is exactly like [`xrange`](StreamCommands::xrange),
    /// but with the notable difference of returning the entries in reverse order,
    /// and also taking the start-end range in reverse order
    ///
    /// # Return
    /// A collection of (`StreamEntry`)[StreamEntry]
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrevrange/>](https://redis.io/commands/xrevrange/)
    fn xrevrange<R: Response>(
        self,
        key: impl Args,
        end: impl Args,
        start: impl Args,
        count: Option<usize>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XREVRANGE")
                .arg(key)
                .arg(end)
                .arg(start)
                .arg(count.map(|c| ("COUNT", c))),
        )
    }

    /// XTRIM trims the stream by evicting older entries (entries with lower IDs) if needed.
    ///
    /// # Return
    /// The number of entries deleted from the stream.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xtrim/>](https://redis.io/commands/xtrim/)
    fn xtrim(self, key: impl Args, options: XTrimOptions) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XTRIM").arg(key).arg(options))
    }
}

/// Stream Add options for the [`xadd`](StreamCommands::xadd) command.
#[derive(Default)]
pub struct XAddOptions {
    command_args: CommandArgs,
}

impl XAddOptions {
    #[must_use]
    pub fn no_mk_stream(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOMKSTREAM").build(),
        }
    }

    #[must_use]
    pub fn trim_options(mut self, trim_options: XTrimOptions) -> Self {
        Self {
            command_args: self.command_args.arg(trim_options).build(),
        }
    }
}

impl Args for XAddOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Stream Trim operator for the [`xadd`](StreamCommands::xadd)
/// and [`xtrim`](StreamCommands::xtrim) commands
#[derive(Default)]
pub enum XTrimOperator {
    #[default]
    None,
    /// =
    Equal,
    /// ~
    Approximately,
}

impl Args for XTrimOperator {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            XTrimOperator::None => {}
            XTrimOperator::Equal => {
                args.arg("=");
            }
            XTrimOperator::Approximately => {
                args.arg("~");
            }
        }
    }
}

/// Stream Trim options for the [`xadd`](StreamCommands::xadd)
/// and [`xtrim`](StreamCommands::xtrim) commands
#[derive(Default)]
pub struct XTrimOptions {
    command_args: CommandArgs,
}

impl XTrimOptions {
    #[must_use]
    pub fn max_len(operator: XTrimOperator, threshold: i64) -> Self {
        Self {
            command_args: CommandArgs::default()
                .arg("MAXLEN")
                .arg(operator)
                .arg(threshold)
                .build(),
        }
    }

    #[must_use]
    pub fn min_id(operator: XTrimOperator, threshold_id: impl Args) -> Self {
        Self {
            command_args: CommandArgs::default()
                .arg("MINID")
                .arg(operator)
                .arg(threshold_id)
                .build(),
        }
    }

    #[must_use]
    pub fn limit(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(count).build(),
        }
    }
}

impl Args for XTrimOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`xautoclaim`](StreamCommands::xautoclaim) command
#[derive(Default)]
pub struct XAutoClaimOptions {
    command_args: CommandArgs,
}

impl XAutoClaimOptions {
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }

    #[must_use]
    pub fn just_id(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("JUSTID").build(),
        }
    }
}

impl Args for XAutoClaimOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`xrange`](StreamCommands::xrange) and other associated commands.
#[derive(Deserialize)]
#[serde(bound = "V: DeserializeOwned")]
pub struct StreamEntry<V>
where
    V: Response,
{
    /// The stream Id
    pub stream_id: String,
    /// entries with their fields and values in the exact same
    /// order as [`xadd`](StreamCommands::xadd) added them.
    pub items: HashMap<String, V>,
}

/// Result for the [`xautoclaim`](StreamCommands::xautoclaim) command.
#[derive(Deserialize)]
#[serde(bound = "V: DeserializeOwned")]
pub struct XAutoClaimResult<V>
where
    V: Response,
{
    /// A stream ID to be used as the `start` argument for
    /// the next call to [`xautoclaim`](StreamCommands::xautoclaim).
    pub start_stream_id: String,
    /// An array containing all the successfully claimed messages in
    /// the same format as [`xrange`](StreamCommands::xrange).
    pub entries: Vec<StreamEntry<V>>,
    /// An array containing message IDs that no longer exist in the stream,
    /// and were deleted from the PEL in which they were found.
    pub deleted_ids: Vec<String>,
}

/// Options for the [`xclaim`](StreamCommands::xclaim) command
#[derive(Default)]
pub struct XClaimOptions {
    command_args: CommandArgs,
}

impl XClaimOptions {
    /// Set the idle time (last time it was delivered) of the message.
    #[must_use]
    pub fn idle_time(mut self, idle_time_millis: u64) -> Self {
        Self {
            command_args: self.command_args.arg("IDLE").arg(idle_time_millis).build(),
        }
    }

    ///  This is the same as `idle_time` but instead of a relative amount of milliseconds,
    /// it sets the idle time to a specific Unix time (in milliseconds).
    #[must_use]
    pub fn time(mut self, unix_time_milliseconds: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("TIME")
                .arg(unix_time_milliseconds)
                .build(),
        }
    }

    /// Set the retry counter to the specified value.
    #[must_use]
    pub fn retry_count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("RETRYCOUNT").arg(count).build(),
        }
    }

    /// Creates the pending message entry in the PEL
    /// even if certain specified IDs are not already
    /// in the PEL assigned to a different client.
    #[must_use]
    pub fn force(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("FORCE").build(),
        }
    }

    ///  Return just an array of IDs of messages successfully claimed,
    /// without returning the actual message.
    #[must_use]
    pub fn just_id(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("JUSTID").build(),
        }
    }
}

impl Args for XClaimOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`xgroup_create`](StreamCommands::xgroup_create) command
#[derive(Default)]
pub struct XGroupCreateOptions {
    command_args: CommandArgs,
}

impl XGroupCreateOptions {
    /// By default, the XGROUP CREATE command insists that the target stream exists and returns an error when it doesn't.
    ///  However, you can use the optional MKSTREAM subcommand as the last argument after the `id`
    /// to automatically create the stream (with length of 0) if it doesn't exist
    #[must_use]
    pub fn mk_stream(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("MKSTREAM").build(),
        }
    }

    /// The optional entries_read named argument can be specified to enable consumer group lag tracking for an arbitrary ID.
    /// An arbitrary ID is any ID that isn't the ID of the stream's first entry, its last entry or the zero ("0-0") ID.
    /// This can be useful you know exactly how many entries are between the arbitrary ID (excluding it) and the stream's last entry.
    /// In such cases, the entries_read can be set to the stream's entries_added subtracted with the number of entries.
    #[must_use]
    pub fn entries_read(mut self, entries_read: usize) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("ENTRIESREAD")
                .arg(entries_read)
                .build(),
        }
    }
}

impl Args for XGroupCreateOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result entry for the [`xinfo_consumers`](StreamCommands::xinfo_consumers) command.
#[derive(Deserialize)]
pub struct XConsumerInfo {
    /// the consumer's name
    pub name: String,

    /// the number of pending messages for the client,
    /// which are messages that were delivered but are yet to be acknowledged
    pub pending: usize,

    /// the number of milliseconds that have passed
    /// since the consumer last interacted with the server
    #[serde(rename = "idle")]
    pub idle_millis: u64,
}

/// Result entry for the [`xinfo_groups`](StreamCommands::xinfo_groups) command.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct XGroupInfo {
    /// the consumer group's name
    pub name: String,

    /// the number of consumers in the group
    pub consumers: usize,

    /// the length of the group's pending entries list (PEL),
    /// which are messages that were delivered but are yet to be acknowledged
    pub pending: usize,

    /// the ID of the last entry delivered the group's consumers
    pub last_delivered_id: String,

    /// the logical "read counter" of the last entry delivered to group's consumers
    pub entries_read: Option<usize>,

    /// the number of entries in the stream that are still waiting to be delivered to the group's consumers,
    /// or a NULL when that number can't be determined.
    pub lag: Option<usize>,
}

/// Options for the [`xinfo_stream`](StreamCommands::xinfo_stream) command
#[derive(Default)]
pub struct XInfoStreamOptions {
    command_args: CommandArgs,
}

impl XInfoStreamOptions {
    /// The optional FULL modifier provides a more verbose reply.
    #[must_use]
    pub fn full(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("FULL").build(),
        }
    }

    /// The COUNT option can be used to limit the number of stream and PEL entries that are returned
    /// (The first `count` entries are returned).
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }
}

impl Args for XInfoStreamOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Stream info returned by the [`xinfo_stream`](StreamCommands::xinfo_stream) command.
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct XStreamInfo {
    /// the number of entries in the stream (see [`xlen`](StreamCommands::xlen))
    pub length: usize,

    /// the number of keys in the underlying radix data structure
    pub radix_tree_keys: usize,

    /// the number of nodes in the underlying radix data structure
    pub radix_tree_nodes: usize,

    /// the number of consumer groups defined for the stream
    pub groups: usize,

    /// the ID of the least-recently entry that was added to the stream
    pub last_generated_id: String,

    /// the maximal entry ID that was deleted from the stream
    pub max_deleted_entry_id: String,

    /// the count of all entries added to the stream during its lifetime
    pub entries_added: usize,

    /// the ID and field-value tuples of the first entry in the stream
    pub first_entry: StreamEntry<String>,

    /// the ID and field-value tuples of the last entry in the stream
    pub last_entry: StreamEntry<String>,

    pub recorded_first_entry_id: String,
}

/// Options for the [`xread`](StreamCommands::xread) command
#[derive(Default)]
pub struct XReadOptions {
    command_args: CommandArgs,
}

impl XReadOptions {
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }

    #[must_use]
    pub fn block(mut self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("BLOCK").arg(milliseconds).build(),
        }
    }
}

impl Args for XReadOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`xreadgroup`](StreamCommands::xreadgroup) command
#[derive(Default)]
pub struct XReadGroupOptions {
    command_args: CommandArgs,
}

impl XReadGroupOptions {
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count).build(),
        }
    }

    #[must_use]
    pub fn block(mut self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("BLOCK").arg(milliseconds).build(),
        }
    }

    #[must_use]
    pub fn no_ack(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOACK").build(),
        }
    }
}

impl Args for XReadGroupOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`xpending_with_options`](StreamCommands::xpending_with_options) command
#[derive(Default)]
pub struct XPendingOptions {
    command_args: CommandArgs,
}

impl XPendingOptions {
    #[must_use]
    pub fn idle(mut self, min_idle_time: u64) -> Self {
        Self {
            command_args: self.command_args.arg("IDLE").arg(min_idle_time).build(),
        }
    }

    #[must_use]
    pub fn start(mut self, start: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg(start).build(),
        }
    }

    #[must_use]
    pub fn end(mut self, end: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg(end).build(),
        }
    }

    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg(count).build(),
        }
    }

    #[must_use]
    pub fn consumer(mut self, consumer: impl Args) -> Self {
        Self {
            command_args: self.command_args.arg(consumer).build(),
        }
    }
}

impl Args for XPendingOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`xpending`](StreamCommands::xpending) command
#[derive(Deserialize)]
pub struct XPendingResult {
    pub num_pending_messages: usize,
    pub smallest_id: String,
    pub greatest_id: String,
    pub consumers: Vec<XPendingConsumer>,
}

/// Customer info result for the [`xpending`](StreamCommands::xpending) command
#[derive(Deserialize)]
pub struct XPendingConsumer {
    pub consumer: String,
    pub num_messages: usize,
}

/// Message result for the [`xpending_with_options`](StreamCommands::xpending_with_options) command
#[derive(Deserialize)]
pub struct XPendingMessageResult {
    pub message_id: String,
    pub consumer: String,
    pub elapsed_millis: u64,
    pub times_delivered: usize,
}
