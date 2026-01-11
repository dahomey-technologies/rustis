use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Response, cmd, serialize_flag},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
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
        key: impl Serialize,
        group: impl Serialize,
        ids: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XACK").key(key).arg(group).arg(ids))
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
        key: impl Serialize,
        stream_id: impl Serialize,
        items: impl Serialize,
        options: XAddOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XADD").key(key).arg(options).arg(stream_id).arg(items),
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
        key: impl Serialize,
        group: impl Serialize,
        consumer: impl Serialize,
        min_idle_time: u64,
        start: impl Serialize,
        options: XAutoClaimOptions,
    ) -> PreparedCommand<'a, Self, XAutoClaimResult<R>> {
        prepare_command(
            self,
            cmd("XAUTOCLAIM")
                .key(key)
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
        key: impl Serialize,
        group: impl Serialize,
        consumer: impl Serialize,
        min_idle_time: u64,
        ids: impl Serialize,
        options: XClaimOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XCLAIM")
                .key(key)
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
    fn xdel(self, key: impl Serialize, ids: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XDEL").key(key).arg(ids))
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
        key: impl Serialize,
        groupname: impl Serialize,
        id: impl Serialize,
        options: XGroupCreateOptions,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("CREATE")
                .key(key)
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
        key: impl Serialize,
        groupname: impl Serialize,
        consumername: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("CREATECONSUMER")
                .key(key)
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
        key: impl Serialize,
        groupname: impl Serialize,
        consumername: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("DELCONSUMER")
                .key(key)
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
        key: impl Serialize,
        groupname: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("XGROUP").arg("DESTROY").key(key).arg(groupname))
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
        key: impl Serialize,
        groupname: impl Serialize,
        id: impl Serialize,
        entries_read: Option<usize>,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("XGROUP")
                .arg("SETID")
                .key(key)
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
        key: impl Serialize,
        groupname: impl Serialize,
    ) -> PreparedCommand<'a, Self, Vec<XConsumerInfo>> {
        prepare_command(self, cmd("XINFO").arg("CONSUMERS").key(key).arg(groupname))
    }

    /// This command returns the list of consumers that belong
    /// to the `groupname` consumer group of the stream stored at `key`.
    ///
    /// # Return
    /// A collection of XGroupInfo.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xinfo-groups/>](https://redis.io/commands/xinfo-groups/)
    fn xinfo_groups(self, key: impl Serialize) -> PreparedCommand<'a, Self, Vec<XGroupInfo>> {
        prepare_command(self, cmd("XINFO").arg("GROUPS").key(key))
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
        key: impl Serialize,
        options: XInfoStreamOptions,
    ) -> PreparedCommand<'a, Self, XStreamInfo> {
        prepare_command(self, cmd("XINFO").arg("STREAM").key(key).arg(options))
    }

    /// Returns the number of entries inside a stream.
    ///
    /// # Return
    /// The number of entries of the stream at `key`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xrange/>](https://redis.io/commands/xrange/)
    fn xlen(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XLEN").key(key))
    }

    /// The XPENDING command is the interface to inspect the list of pending messages.
    ///
    /// # See Also
    /// [<https://redis.io/commands/xpending/>](https://redis.io/commands/xpending/)
    fn xpending(
        self,
        key: impl Serialize,
        group: impl Serialize,
    ) -> PreparedCommand<'a, Self, XPendingResult> {
        prepare_command(self, cmd("XPENDING").key(key).arg(group))
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
        key: impl Serialize,
        group: impl Serialize,
        options: XPendingOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("XPENDING").key(key).arg(group).arg(options))
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
        key: impl Serialize,
        start: impl Serialize,
        end: impl Serialize,
        count: Option<usize>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XRANGE")
                .key(key)
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
        keys: impl Serialize,
        ids: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XREAD").arg(options).arg("STREAMS").key(keys).arg(ids),
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
        group: impl Serialize,
        consumer: impl Serialize,
        options: XReadGroupOptions,
        keys: impl Serialize,
        ids: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XREADGROUP")
                .arg("GROUP")
                .arg(group)
                .arg(consumer)
                .arg(options)
                .arg("STREAMS")
                .key(keys)
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
        key: impl Serialize,
        end: impl Serialize,
        start: impl Serialize,
        count: Option<usize>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("XREVRANGE")
                .key(key)
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
    fn xtrim(self, key: impl Serialize, options: XTrimOptions) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("XTRIM").key(key).arg(options))
    }
}

/// Consumer group options for the [`xadd`](StreamCommands::xadd) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConsumerGroupOptions {
    /// When trimming, removes entries from the stream according to the specified strategy (MAXLEN or MINID),
    /// regardless of whether they are referenced by any consumer groups, but preserves existing references
    /// to these entries in all consumer groups' PEL (Pending Entries List).
    KeepRef,
    /// When trimming, removes entries from the stream according to the specified strategy
    /// and also removes all references to these entries from all consumer groups' PEL.
    DelRef,
    /// When trimming, only removes entries that were read and acknowledged by all consumer groups.
    /// Note that if the number of referenced entries is larger than MAXLEN,
    /// trimming will still stop at the limit.
    Acked,
}

/// Stream Add options for the [`xadd`](StreamCommands::xadd) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XAddOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nomkstream: bool,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    consumer_group_options: Option<ConsumerGroupOptions>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    trim_options: Option<XTrimOptions<'a>>,
}

impl<'a> XAddOptions<'a> {
    #[must_use]
    pub fn no_mk_stream(mut self) -> Self {
        self.nomkstream = true;
        self
    }

    #[must_use]
    pub fn consumer_group_options(mut self, consumer_group_options: ConsumerGroupOptions) -> Self {
        self.consumer_group_options = Some(consumer_group_options);
        self
    }

    #[must_use]
    pub fn trim_options(mut self, trim_options: XTrimOptions<'a>) -> Self {
        self.trim_options = Some(trim_options);
        self
    }
}

/// Stream Trim operator for the [`xadd`](StreamCommands::xadd)
/// and [`xtrim`](StreamCommands::xtrim) commands
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum XTrimOperator {
    /// =
    Equal,
    /// ~
    Approximately,
}

/// Stream Trim options for the [`xadd`](StreamCommands::xadd)
/// and [`xtrim`](StreamCommands::xtrim) commands
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XTrimOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    maxlen: Option<(Option<XTrimOperator>, i64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minid: Option<(Option<XTrimOperator>, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
}

impl<'a> XTrimOptions<'a> {
    #[must_use]
    pub fn max_len(operator: impl Into<Option<XTrimOperator>>, threshold: i64) -> Self {
        Self {
            maxlen: Some((operator.into(), threshold)),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn min_id(operator: impl Into<Option<XTrimOperator>>, threshold_id: &'a str) -> Self {
        Self {
            minid: Some((operator.into(), threshold_id)),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn limit(mut self, count: u32) -> Self {
        self.limit = Some(count);
        self
    }
}

/// Options for the [`xautoclaim`](StreamCommands::xautoclaim) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XAutoClaimOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    justid: bool,
}

impl XAutoClaimOptions {
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    #[must_use]
    pub fn just_id(mut self) -> Self {
        self.justid = true;
        self
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
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XClaimOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    idle: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retrycount: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    force: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    justid: bool,
}

impl XClaimOptions {
    /// Set the idle time (last time it was delivered) of the message.
    #[must_use]
    pub fn idle_time(mut self, ms: u64) -> Self {
        self.idle = Some(ms);
        self
    }

    ///  This is the same as `idle_time` but instead of a relative amount of milliseconds,
    /// it sets the idle time to a specific Unix time (in milliseconds).
    #[must_use]
    pub fn time(mut self, unix_time_milliseconds: u64) -> Self {
        self.time = Some(unix_time_milliseconds);
        self
    }

    /// Set the retry counter to the specified value.
    #[must_use]
    pub fn retry_count(mut self, count: u32) -> Self {
        self.retrycount = Some(count);
        self
    }

    /// Creates the pending message entry in the PEL
    /// even if certain specified IDs are not already
    /// in the PEL assigned to a different client.
    #[must_use]
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Return just an array of IDs of messages successfully claimed,
    /// without returning the actual message.
    #[must_use]
    pub fn just_id(mut self) -> Self {
        self.justid = true;
        self
    }
}

/// Options for the [`xgroup_create`](StreamCommands::xgroup_create) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XGroupCreateOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    mkstream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    entriesread: Option<u32>,
}

impl XGroupCreateOptions {
    /// By default, the XGROUP CREATE command insists that the target stream exists and returns an error when it doesn't.
    ///  However, you can use the optional MKSTREAM subcommand as the last argument after the `id`
    /// to automatically create the stream (with length of 0) if it doesn't exist
    #[must_use]
    pub fn mk_stream(mut self) -> Self {
        self.mkstream = true;
        self
    }

    /// The optional entries_read named argument can be specified to enable consumer group lag tracking for an arbitrary ID.
    /// An arbitrary ID is any ID that isn't the ID of the stream's first entry, its last entry or the zero ("0-0") ID.
    /// This can be useful you know exactly how many entries are between the arbitrary ID (excluding it) and the stream's last entry.
    /// In such cases, the entries_read can be set to the stream's entries_added subtracted with the number of entries.
    #[must_use]
    pub fn entries_read(mut self, entries_read: u32) -> Self {
        self.entriesread = Some(entries_read);
        self
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
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XInfoStreamOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    full: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
}

impl XInfoStreamOptions {
    /// The optional FULL modifier provides a more verbose reply.
    #[must_use]
    pub fn full() -> Self {
        Self {
            full: true,
            ..Default::default()
        }
    }

    /// The COUNT option can be used to limit the number of stream and PEL entries that are returned
    /// (The first `count` entries are returned).
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
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
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XReadOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block: Option<u64>,
}

impl XReadOptions {
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    #[must_use]
    pub fn block(mut self, milliseconds: u64) -> Self {
        self.block = Some(milliseconds);
        self
    }
}

/// Options for the [`xreadgroup`](StreamCommands::xreadgroup) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XReadGroupOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    claim: Option<u64>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    noack: bool,
}

impl XReadGroupOptions {
    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    #[must_use]
    pub fn block(mut self, milliseconds: u64) -> Self {
        self.block = Some(milliseconds);
        self
    }

    /// When CLAIM min-idle-time is specified, Redis will first try to claim messages which have been pending
    /// for at least min-idle-time milliseconds from the consumer group of each specified stream key.
    /// The pending messages with the highest idle time would be claimed first. Note that the CLAIM min-idle-time
    /// condition may become true for some pending entries during the BLOCK milliseconds period (if specified).
    ///
    /// If there are no such messages, Redis will continue as normal (consume incoming messages).
    ///
    /// CLAIM min-idle-time is ignored if the specified id is not >.
    #[must_use]
    pub fn claim(mut self, min_idle_time: u64) -> Self {
        self.claim = Some(min_idle_time);
        self
    }

    /// The NOACK subcommand can be used to avoid adding the message to the PEL in cases where reliability
    /// is not a requirement and the occasional message loss is acceptable.
    /// This is equivalent to acknowledging the message when it is read.
    /// When used together with CLAIM, NOACK does not apply for retrieved pending entries.
    #[must_use]
    pub fn no_ack(mut self) -> Self {
        self.noack = true;
        self
    }
}

/// Options for the [`xpending_with_options`](StreamCommands::xpending_with_options) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct XPendingOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    idle: Option<u64>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    start: Option<&'a str>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    end: Option<&'a str>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    consumer: Option<&'a str>,
}

impl<'a> XPendingOptions<'a> {
    #[must_use]
    pub fn idle(mut self, min_idle_time: u64) -> Self {
        self.idle = Some(min_idle_time);
        self
    }

    #[must_use]
    pub fn start(mut self, start: &'a str) -> Self {
        self.start = Some(start);
        self
    }

    #[must_use]
    pub fn end(mut self, end: &'a str) -> Self {
        self.end = Some(end);
        self
    }

    #[must_use]
    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    #[must_use]
    pub fn consumer(mut self, consumer: &'a str) -> Self {
        self.consumer = Some(consumer);
        self
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
