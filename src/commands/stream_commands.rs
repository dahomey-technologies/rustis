use std::collections::HashMap;

use crate::{
    resp::{
        cmd, BulkString, CommandArgs, FromValue, HashMapExt, IntoArgs, KeyValueArgOrCollection,
        SingleArgOrCollection, Value,
    },
    CommandResult, PrepareCommand, Result,
};

/// A group of Redis commands related to [`Streams`](https://redis.io/docs/data-types/streams/)
/// # See Also
/// [Redis Generic Commands](https://redis.io/commands/?group=stream)
/// [Streams tutorial](https://redis.io/docs/data-types/streams-tutorial/)
pub trait StreamCommands<T>: PrepareCommand<T> {
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
    fn xack<K, G, I, II>(&self, key: K, group: G, ids: II) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        I: Into<BulkString>,
        II: SingleArgOrCollection<I>,
    {
        self.prepare_command(cmd("XACK").arg(key).arg(group).arg(ids))
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
    fn xadd<K, I, F, V, FFVV, R>(
        &self,
        key: K,
        stream_id: I,
        items: FFVV,
        options: XAddOptions,
    ) -> CommandResult<T, R>
    where
        K: Into<BulkString>,
        I: Into<BulkString>,
        F: Into<BulkString>,
        V: Into<BulkString>,
        FFVV: KeyValueArgOrCollection<F, V>,
        R: FromValue,
    {
        self.prepare_command(cmd("XADD").arg(key).arg(options).arg(stream_id).arg(items))
    }

    /// This command transfers ownership of pending stream entries that match the specified criteria.
    ///
    /// # Return
    /// An instance of StreamAutoClaimResult
    ///
    /// # See Also
    /// [<https://redis.io/commands/xautoclaim/>](https://redis.io/commands/xautoclaim/)
    fn xautoclaim<K, G, C, I, V>(
        &self,
        key: K,
        group: G,
        consumer: C,
        min_idle_time: u64,
        start: I,
        options: XAutoClaimOptions,
    ) -> CommandResult<T, XAutoClaimResult<V>>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        C: Into<BulkString>,
        I: Into<BulkString>,
        V: FromValue,
    {
        self.prepare_command(
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
    fn xclaim<K, G, C, I, II, V>(
        &self,
        key: K,
        group: G,
        consumer: C,
        min_idle_time: u64,
        ids: II,
        options: XClaimOptions,
    ) -> CommandResult<T, Vec<StreamEntry<V>>>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        C: Into<BulkString>,
        I: Into<BulkString>,
        II: SingleArgOrCollection<I>,
        V: FromValue,
    {
        self.prepare_command(
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
    fn xdel<K, I, II>(&self, key: K, ids: II) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        I: Into<BulkString>,
        II: SingleArgOrCollection<I>,
    {
        self.prepare_command(cmd("XDEL").arg(key).arg(ids))
    }

    /// This command creates a new consumer group uniquely identified by <groupname> for the stream stored at <key>.
    ///
    /// # Return
    /// * `true` success
    /// * `false`failure
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-create/>](https://redis.io/commands/xgroup-create/)
    fn xgroup_create<K, G, I>(
        &self,
        key: K,
        groupname: G,
        id: I,
        options: XGroupCreateOptions,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        I: Into<BulkString>,
    {
        self.prepare_command(
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
    fn xgroup_createconsumer<K, G, C>(
        &self,
        key: K,
        groupname: G,
        consumername: C,
    ) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        C: Into<BulkString>,
    {
        self.prepare_command(
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
    fn xgroup_delconsumer<K, G, C>(
        &self,
        key: K,
        groupname: G,
        consumername: C,
    ) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        C: Into<BulkString>,
    {
        self.prepare_command(
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
    fn xgroup_destroy<K, G>(&self, key: K, groupname: G) -> CommandResult<T, bool>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
    {
        self.prepare_command(cmd("XGROUP").arg("DESTROY").arg(key).arg(groupname))
    }

    /// Set the last delivered ID for a consumer group.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xgroup-setid/>](https://redis.io/commands/xgroup-setid/)
    fn xgroup_setid<K, G, I>(
        &self,
        key: K,
        groupname: G,
        id: I,
        entries_read: Option<usize>,
    ) -> CommandResult<T, ()>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
        I: Into<BulkString>,
    {
        self.prepare_command(
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
    fn xinfo_consumers<K, G>(&self, key: K, groupname: G) -> CommandResult<T, Vec<XConsumerInfo>>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
    {
        self.prepare_command(cmd("XINFO").arg("CONSUMERS").arg(key).arg(groupname))
    }

    /// This command returns the list of consumers that belong
    /// to the `groupname` consumer group of the stream stored at `key`.
    ///
    /// # Return
    /// A collection of XGroupInfo.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-groups/>](https://redis.io/commands/xinfo-groups/)
    fn xinfo_groups<K>(&self, key: K) -> CommandResult<T, Vec<XGroupInfo>>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("XINFO").arg("GROUPS").arg(key))
    }

    /// This command returns information about the stream stored at `key`.
    ///
    /// # Return
    /// A collection of XGroupInfo.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-stream/>](https://redis.io/commands/xinfo-stream/)
    fn xinfo_stream<K>(&self, key: K, options: XInfoStreamOptions) -> CommandResult<T, XStreamInfo>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("XINFO").arg("STREAM").arg(key).arg(options))
    }

    /// Returns the number of entries inside a stream.
    ///
    /// # Return
    /// The number of entries of the stream at `key`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrange/>](https://redis.io/commands/xrange/)
    fn xlen<K>(&self, key: K) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("XLEN").arg(key))
    }

    /// The XPENDING command is the interface to inspect the list of pending messages.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xpending/>](https://redis.io/commands/xpending/)
    fn xpending<K, G>(&self, key: K, group: G) -> CommandResult<T, XPendingResult>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
    {
        self.prepare_command(cmd("XPENDING").arg(key).arg(group))
    }

    /// The XPENDING command is the interface to inspect the list of pending messages.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xpending/>](https://redis.io/commands/xpending/)
    fn xpending_with_options<K, G>(
        &self,
        key: K,
        group: G,
        options: XPendingOptions,
    ) -> CommandResult<T, Vec<XPendingMessageResult>>
    where
        K: Into<BulkString>,
        G: Into<BulkString>,
    {
        self.prepare_command(cmd("XPENDING").arg(key).arg(group).arg(options))
    }

    /// The command returns the stream entries matching a given range of IDs.
    ///
    /// # Return
    /// A collection of StreamEntry
    ///
    /// The command returns the entries with IDs matching the specified range.
    /// The returned entries are complete, that means that the ID and all the fields they are composed are returned.
    /// Moreover, the entries are returned with their fields and values in the exact same order as XADD added them.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrange/>](https://redis.io/commands/xrange/)
    fn xrange<K, S, E, V>(
        &self,
        key: K,
        start: S,
        end: E,
        count: Option<usize>,
    ) -> CommandResult<T, Vec<StreamEntry<V>>>
    where
        K: Into<BulkString>,
        S: Into<BulkString>,
        E: Into<BulkString>,
        V: FromValue,
    {
        self.prepare_command(
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
    /// A collection of XReadStreamResult
    ///
    /// # See Also
    /// [<https://redis.io/commands/xread/>](https://redis.io/commands/xread/)
    fn xread<K, KK, I, II, V>(
        &self,
        options: XReadOptions,
        keys: KK,
        ids: II,
    ) -> CommandResult<T, Vec<XReadStreamResult<V>>>
    where
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        I: Into<BulkString>,
        II: SingleArgOrCollection<I>,
        V: FromValue,
    {
        self.prepare_command(cmd("XREAD").arg(options).arg("STREAMS").arg(keys).arg(ids))
    }

    /// The XREADGROUP command is a special version of the [`xread`](crate::StreamCommands::xread)
    /// command with support for consumer groups.
    ///
    /// # Return
    /// A collection of XReadStreamResult
    ///
    /// # See Also
    /// [<https://redis.io/commands/xreadgroup/>](https://redis.io/commands/xreadgroup/)
    fn xreadgroup<G, C, K, KK, I, II, V>(
        &self,
        group: G,
        consumer: C,
        options: XReadGroupOptions,
        keys: KK,
        ids: II,
    ) -> CommandResult<T, Vec<XReadStreamResult<V>>>
    where
        G: Into<BulkString>,
        C: Into<BulkString>,
        K: Into<BulkString>,
        KK: SingleArgOrCollection<K>,
        I: Into<BulkString>,
        II: SingleArgOrCollection<I>,
        V: FromValue,
    {
        self.prepare_command(
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

    /// This command is exactly like [`xrange`](crate::StreamCommands::xrange),
    /// but with the notable difference of returning the entries in reverse order,
    /// and also taking the start-end range in reverse order
    ///
    /// # Return
    /// A collection of StreamEntry
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrevrange/>](https://redis.io/commands/xrevrange/)
    fn xrevrange<K, E, S, V>(
        &self,
        key: K,
        end: E,
        start: S,
        count: Option<usize>,
    ) -> CommandResult<T, Vec<StreamEntry<V>>>
    where
        K: Into<BulkString>,
        E: Into<BulkString>,
        S: Into<BulkString>,
        V: FromValue,
    {
        self.prepare_command(
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
    fn xtrim<K>(&self, key: K, options: XTrimOptions) -> CommandResult<T, usize>
    where
        K: Into<BulkString>,
    {
        self.prepare_command(cmd("XTRIM").arg(key).arg(options))
    }
}

/// Stream Add options for the [`xadd`](crate::StreamCommands::xadd) command.
#[derive(Default)]
pub struct XAddOptions {
    command_args: CommandArgs,
}

impl XAddOptions {
    #[must_use]
    pub fn no_mk_stream(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOMKSTREAM"),
        }
    }

    #[must_use]
    pub fn trim_options(self, trim_options: XTrimOptions) -> Self {
        Self {
            command_args: self.command_args.arg(trim_options),
        }
    }
}

impl IntoArgs for XAddOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Stream Trim operator for the [`xadd`](crate::StreamCommands::xadd)
/// and [`xtrim`](crate::StreamCommands::xtrim) commands
pub enum XTrimOperator {
    None,
    /// =
    Equal,
    /// ~
    Approximately,
}

impl IntoArgs for XTrimOperator {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        match self {
            XTrimOperator::None => args,
            XTrimOperator::Equal => args.arg(BulkString::Str("=")),
            XTrimOperator::Approximately => args.arg(BulkString::Str("~")),
        }
    }
}

impl Default for XTrimOperator {
    fn default() -> Self {
        XTrimOperator::None
    }
}

/// Stream Trim options for the [`xadd`](crate::StreamCommands::xadd)
/// and [`xtrim`](crate::StreamCommands::xtrim) commands
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
                .arg(threshold),
        }
    }

    #[must_use]
    pub fn min_id<I: Into<BulkString>>(operator: XTrimOperator, threshold_id: I) -> Self {
        Self {
            command_args: CommandArgs::default()
                .arg("MINID")
                .arg(operator)
                .arg(threshold_id),
        }
    }

    #[must_use]
    pub fn limit(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(count),
        }
    }
}

impl IntoArgs for XTrimOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`xautoclaim`](crate::StreamCommands::xautoclaim) command
#[derive(Default)]
pub struct XAutoClaimOptions {
    command_args: CommandArgs,
}

impl XAutoClaimOptions {
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }

    #[must_use]
    pub fn just_id(self) -> Self {
        Self {
            command_args: self.command_args.arg("JUSTID"),
        }
    }
}

impl IntoArgs for XAutoClaimOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

pub struct StreamEntry<V>
where
    V: FromValue,
{
    pub stream_id: String,
    pub items: HashMap<String, V>,
}

impl<V> FromValue for StreamEntry<V>
where
    V: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        let (stream_id, items): (String, HashMap<String, V>) = value.into()?;
        Ok(Self { stream_id, items })
    }
}

pub struct XAutoClaimResult<V>
where
    V: FromValue,
{
    pub start_stream_id: String,
    pub entries: Vec<StreamEntry<V>>,
    pub deleted_id: Vec<String>,
}

impl<V> FromValue for XAutoClaimResult<V>
where
    V: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        let (start_stream_id, entries, deleted_id): (String, Vec<StreamEntry<V>>, Vec<String>) =
            value.into()?;
        Ok(Self {
            start_stream_id,
            entries,
            deleted_id,
        })
    }
}

/// Options for the [`xclaim`](crate::StreamCommands::xclaim) command
#[derive(Default)]
pub struct XClaimOptions {
    command_args: CommandArgs,
}

impl XClaimOptions {
    /// Set the idle time (last time it was delivered) of the message.
    #[must_use]
    pub fn idle_time(self, idle_time_millis: u64) -> Self {
        Self {
            command_args: self.command_args.arg("IDLE").arg(idle_time_millis),
        }
    }

    ///  This is the same as `idle_time` but instead of a relative amount of milliseconds,
    /// it sets the idle time to a specific Unix time (in milliseconds).
    #[must_use]
    pub fn time(self, unix_time_milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TIME").arg(unix_time_milliseconds),
        }
    }

    /// Set the retry counter to the specified value.
    #[must_use]
    pub fn retry_count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("RETRYCOUNT").arg(count),
        }
    }

    /// Creates the pending message entry in the PEL
    /// even if certain specified IDs are not already
    /// in the PEL assigned to a different client.
    #[must_use]
    pub fn force(self) -> Self {
        Self {
            command_args: self.command_args.arg("FORCE"),
        }
    }

    ///  Return just an array of IDs of messages successfully claimed,
    /// without returning the actual message.
    #[must_use]
    pub fn just_id(self) -> Self {
        Self {
            command_args: self.command_args.arg("JUSTID"),
        }
    }
}

impl IntoArgs for XClaimOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`xgroup_create`](crate::StreamCommands::xgroup_create) command
#[derive(Default)]
pub struct XGroupCreateOptions {
    command_args: CommandArgs,
}

impl XGroupCreateOptions {
    /// By default, the XGROUP CREATE command insists that the target stream exists and returns an error when it doesn't.
    ///  However, you can use the optional MKSTREAM subcommand as the last argument after the `id`
    /// to automatically create the stream (with length of 0) if it doesn't exist
    #[must_use]
    pub fn mk_stream(self) -> Self {
        Self {
            command_args: self.command_args.arg("MKSTREAM"),
        }
    }

    /// The optional entries_read named argument can be specified to enable consumer group lag tracking for an arbitrary ID.
    /// An arbitrary ID is any ID that isn't the ID of the stream's first entry, its last entry or the zero ("0-0") ID.
    /// This can be useful you know exactly how many entries are between the arbitrary ID (excluding it) and the stream's last entry.
    /// In such cases, the entries_read can be set to the stream's entries_added subtracted with the number of entries.
    #[must_use]
    pub fn entries_read(self, entries_read: usize) -> Self {
        Self {
            command_args: self.command_args.arg("ENTRIESREAD").arg(entries_read),
        }
    }
}

impl IntoArgs for XGroupCreateOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result entry for the [`xinfo_consumers`](crate::StreamCommands::xinfo_consumers) command.
pub struct XConsumerInfo {
    /// the consumer's name
    pub name: String,

    /// the number of pending messages for the client,
    /// which are messages that were delivered but are yet to be acknowledged
    pub pending: usize,

    /// the number of milliseconds that have passed
    /// since the consumer last interacted with the server
    pub idle_millis: u64,
}

impl FromValue for XConsumerInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            name: values.remove_with_result("name")?.into()?,
            pending: values.remove_with_result("pending")?.into()?,
            idle_millis: values.remove_with_result("idle")?.into()?,
        })
    }
}

/// Result entry for the [`xinfo_groups`](crate::StreamCommands::xinfo_groups) command.
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

impl FromValue for XGroupInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            name: values.remove_with_result("name")?.into()?,
            consumers: values.remove_with_result("consumers")?.into()?,
            pending: values.remove_with_result("pending")?.into()?,
            last_delivered_id: values.remove_with_result("last-delivered-id")?.into()?,
            entries_read: values.remove_with_result("entries-read")?.into()?,
            lag: values.remove_with_result("lag")?.into()?,
        })
    }
}

/// Options for the [`xinfo_stream`](crate::StreamCommands::xinfo_stream) command
#[derive(Default)]
pub struct XInfoStreamOptions {
    command_args: CommandArgs,
}

impl XInfoStreamOptions {
    /// The optional FULL modifier provides a more verbose reply.
    #[must_use]
    pub fn full(self) -> Self {
        Self {
            command_args: self.command_args.arg("FULL"),
        }
    }

    /// The COUNT option can be used to limit the number of stream and PEL entries that are returned
    /// (The first `count` entries are returned).
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }
}

impl IntoArgs for XInfoStreamOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Stream info returned by the [`xinfo_stream`](crate::StreamCommands::xinfo_stream) command.
pub struct XStreamInfo {
    /// the number of entries in the stream (see [`xlen`](crate::StreamCommands::xlen))
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

impl FromValue for XStreamInfo {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            length: values.remove_with_result("length")?.into()?,
            radix_tree_keys: values.remove_with_result("radix-tree-keys")?.into()?,
            radix_tree_nodes: values.remove_with_result("radix-tree-nodes")?.into()?,
            groups: values.remove_with_result("groups")?.into()?,
            last_generated_id: values.remove_with_result("last-generated-id")?.into()?,
            max_deleted_entry_id: values.remove_with_result("max-deleted-entry-id")?.into()?,
            entries_added: values.remove_with_result("entries-added")?.into()?,
            first_entry: values.remove_with_result("first-entry")?.into()?,
            last_entry: values.remove_with_result("last-entry")?.into()?,
            recorded_first_entry_id: values
                .remove_with_result("recorded-first-entry-id")?
                .into()?,
        })
    }
}

/// Options for the [`xread`](crate::StreamCommands::xread) command
#[derive(Default)]
pub struct XReadOptions {
    command_args: CommandArgs,
}

impl XReadOptions {
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }

    #[must_use]
    pub fn block(self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("BLOCK").arg(milliseconds),
        }
    }
}

impl IntoArgs for XReadOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`xread`](crate::StreamCommands::xread) command.
pub struct XReadStreamResult<V>
where
    V: FromValue,
{
    pub key: String,
    pub entries: Vec<StreamEntry<V>>,
}

impl<V> FromValue for XReadStreamResult<V>
where
    V: FromValue,
{
    fn from_value(value: Value) -> Result<Self> {
        let (key, entries): (String, Vec<StreamEntry<V>>) = value.into()?;

        Ok(Self { key, entries })
    }
}

/// Options for the [`xreadgroup`](crate::StreamCommands::xreadgroup) command
#[derive(Default)]
pub struct XReadGroupOptions {
    command_args: CommandArgs,
}

impl XReadGroupOptions {
    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(count),
        }
    }

    #[must_use]
    pub fn block(self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("BLOCK").arg(milliseconds),
        }
    }

    #[must_use]
    pub fn no_ack(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOACK"),
        }
    }
}

impl IntoArgs for XReadGroupOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`xpending_with_options`](crate::StreamCommands::xpending_with_options) command
#[derive(Default)]
pub struct XPendingOptions {
    command_args: CommandArgs,
}

impl XPendingOptions {
    #[must_use]
    pub fn idle(self, min_idle_time: u64) -> Self {
        Self {
            command_args: self.command_args.arg("IDLE").arg(min_idle_time),
        }
    }

    #[must_use]
    pub fn start<S: Into<BulkString>>(self, start: S) -> Self {
        Self {
            command_args: self.command_args.arg(start),
        }
    }

    #[must_use]
    pub fn end<E: Into<BulkString>>(self, end: E) -> Self {
        Self {
            command_args: self.command_args.arg(end),
        }
    }

    #[must_use]
    pub fn count(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg(count),
        }
    }

    #[must_use]
    pub fn consumer<C: Into<BulkString>>(self, consumer: C) -> Self {
        Self {
            command_args: self.command_args.arg(consumer),
        }
    }
}

impl IntoArgs for XPendingOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`xpending`](crate::StreamCommands::xpending) command
pub struct XPendingResult {
    pub num_pending_messages: usize,
    pub smallest_id: String,
    pub greatest_id: String,
    pub consumers: Vec<XPendingConsumer>,
}

impl FromValue for XPendingResult {
    fn from_value(value: Value) -> Result<Self> {
        let (num_pending_messages, smallest_id, greatest_id, consumers): (
            usize,
            String,
            String,
            Vec<XPendingConsumer>,
        ) = value.into()?;
        Ok(Self {
            num_pending_messages,
            smallest_id,
            greatest_id,
            consumers,
        })
    }
}

/// Customer info result for the [`xpending`](crate::StreamCommands::xpending) command
pub struct XPendingConsumer {
    pub consumer: String,
    pub num_messages: usize,
}

impl FromValue for XPendingConsumer {
    fn from_value(value: Value) -> Result<Self> {
        let (consumer, num_messages): (String, usize) = value.into()?;
        Ok(Self {
            consumer,
            num_messages,
        })
    }
}

/// Message result for the [`xpending_with_options`](crate::StreamCommands::xpending_with_options) command
pub struct XPendingMessageResult {
    pub message_id: String,
    pub consumer: String,
    pub elapsed_millis: u64,
    pub times_delivered: usize,
}

impl FromValue for XPendingMessageResult {
    fn from_value(value: Value) -> Result<Self> {
        let (message_id, consumer, elapsed_millis, times_delivered): (String, String, u64, usize) =
            value.into()?;
        Ok(Self {
            message_id,
            consumer,
            elapsed_millis,
            times_delivered,
        })
    }
}
