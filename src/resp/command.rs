use crate::{
    ClientError, Error,
    commands::{RequestPolicy, ResponsePolicy},
    resp::{ArgCounter, ArgSerializer},
};
use bytes::{BufMut, Bytes, BytesMut};
use memchr::memchr;
use serde::Serialize;
use smallvec::SmallVec;
#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
};

#[cfg(test)]
static COMMAND_SEQUENCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The size in bytes reserved at the beginning of the buffer.
///
/// It provides enough space to write the RESP command header
/// (e.g., `*3\r\n`) *after* the command name & arguments have been serialized,
/// avoiding memory moves or additional allocations.
const HEADROOM_SIZE: usize = 16;

/// Shortcut function for creating a command.
///
/// # Cluster routing
/// Arguments that are Redis keys must be added with
/// [`CommandBuilder::key`], not [`CommandBuilder::arg`]: only `key`
/// arguments take part in slot computation. A command carrying no key is
/// sent to a random node of the cluster, which silently reads or writes the
/// wrong shard. See the [`key`](CommandBuilder::key) documentation.
#[must_use]
#[inline(always)]
pub fn cmd(name: &'static str) -> CommandBuilder {
    CommandBuilder::new(name.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubscriptionType {
    Channel,
    Pattern,
    ShardChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClientReplyMode {
    On,
    Off,
    Skip,
}

/// One piece of state that lives in the connection rather than in the server, and
/// that a new socket therefore starts without.
///
/// The declaration order is the order in which the slots are replayed after a
/// reconnection: `Auth` first, because every following command runs as the
/// identity it establishes, then `Select`, which decides the keyspace.
///
/// `READONLY` / `READWRITE` is deliberately absent. It only means anything on a
/// cluster, where the replay runs once per node — so a slot for it would broadcast
/// a capability the send path refuses to broadcast, since slot-based reads always
/// go to the shard's master. On a standalone server the command is refused outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StateSlot {
    Auth,
    Select,
    Name,
    LibName,
    LibVer,
    NoEvict,
    NoTouch,
    Tracking,
    ScriptDebug,
    /// `CLIENT REPLY ON` / `OFF`. `SKIP` is not a slot: it applies to one
    /// command, not to the connection.
    ReplyMode,
}

impl StateSlot {
    pub(crate) const ALL: [StateSlot; 10] = [
        StateSlot::Auth,
        StateSlot::Select,
        StateSlot::Name,
        StateSlot::LibName,
        StateSlot::LibVer,
        StateSlot::NoEvict,
        StateSlot::NoTouch,
        StateSlot::Tracking,
        StateSlot::ScriptDebug,
        StateSlot::ReplyMode,
    ];

    pub(crate) fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandKind {
    Other,
    Unsbuscribe(SubscriptionType),
    ClientReply(ClientReplyMode),
    /// A command whose effect is attached to the connection, and which must
    /// therefore be replayed when the connection is remade.
    ConnectionState(StateSlot),
    Reset,
}

/// Represents the memory layout and metadata of a single Redis command argument.
///
/// This structure is packed into 96 bits (12 bytes) to minimize its footprint.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub(crate) struct ArgLayout {
    /// The starting position of the argument data within the command's internal
    /// buffer. A single command's buffer never nears 4 GiB (Redis caps a bulk
    /// string at 512 MiB), so `u32` is sufficient and keeps the struct at 12 B.
    pub start: u32,

    /// The length of the argument data in bytes.
    /// Redis limits bulk strings to 512MB, so `u32` is more than sufficient.
    pub len: u32,

    /// The CRC16 hash slot (0-16383) of this argument, when it is a key.
    /// Populated on the *caller* thread by [`Command::compute_slots`], and only
    /// when the client runs in Cluster mode, so the shared network thread keeps
    /// O(1) routing and standalone clients pay no CRC16. Left at 0 otherwise.
    pub slot: u16,

    /// Bitwise flags for argument properties.
    /// - Bit 0: `IS_KEY` (indicates if the argument is a Redis key for routing).
    /// - Remaining bits: Reserved for future use (e.g., compression, encryption).
    pub flags: u16,
}

/// Inline capacity of a command's argument-layout list.
///
/// Every [`Command`] carries this array inline, and a `Command` is moved
/// (memcpy'd) several times per request along the serialized network path, so
/// this capacity is a direct throughput knob: it costs
/// `ARGS_LAYOUT_INLINE * 12` bytes on every command, whatever its arity.
/// Commands with more arguments spill to the heap on the *caller* thread, which
/// parallelizes; the inline bytes would instead be copied on the shared network
/// task.
pub(crate) const ARGS_LAYOUT_INLINE: usize = 4;

/// Argument-layout list of a command, sized by [`ARGS_LAYOUT_INLINE`].
pub(crate) type ArgsLayout = SmallVec<[ArgLayout; ARGS_LAYOUT_INLINE]>;

#[expect(
    clippy::arithmetic_side_effects,
    clippy::cast_possible_truncation,
    reason = "invariant: a layout is built from a range the builder just wrote into \
              the command buffer, so its end is at or past its start, and the whole \
              buffer fits `u32` — Redis caps a bulk string at 512 MiB, as the field \
              documentation above states."
)]
impl ArgLayout {
    /// Flag indicating that this argument is a Redis key.
    const IS_KEY: u16 = 1 << 0;

    #[inline(always)]
    pub(crate) fn arg(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start as u32,
            len: range.end as u32 - range.start as u32,
            slot: 0,
            flags: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn key(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start as u32,
            len: range.end as u32 - range.start as u32,
            slot: 0,
            flags: Self::IS_KEY,
        }
    }

    #[inline(always)]
    pub(crate) fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.start as usize + self.len as usize
    }

    #[inline(always)]
    pub(crate) fn is_key(&self) -> bool {
        self.flags & Self::IS_KEY != 0
    }

    #[inline(always)]
    pub(crate) fn set_key(&mut self) {
        self.flags |= Self::IS_KEY;
    }
}

impl<'a> From<&'a Command> for CommandKind {
    fn from(command: &'a Command) -> Self {
        match command.name() {
            b"UNSUBSCRIBE" => CommandKind::Unsbuscribe(SubscriptionType::Channel),
            b"PUNSUBSCRIBE" => CommandKind::Unsbuscribe(SubscriptionType::Pattern),
            b"SUNSUBSCRIBE" => CommandKind::Unsbuscribe(SubscriptionType::ShardChannel),
            b"CLIENT" => match (command.get_arg(0).as_deref(), command.get_arg(1).as_deref()) {
                (Some(b"REPLY"), Some(b"ON")) => CommandKind::ClientReply(ClientReplyMode::On),
                (Some(b"REPLY"), Some(b"OFF")) => CommandKind::ClientReply(ClientReplyMode::Off),
                (Some(b"REPLY"), Some(b"SKIP")) => CommandKind::ClientReply(ClientReplyMode::Skip),
                (Some(b"SETNAME"), _) => CommandKind::ConnectionState(StateSlot::Name),
                (Some(b"SETINFO"), Some(b"LIB-NAME")) => {
                    CommandKind::ConnectionState(StateSlot::LibName)
                }
                (Some(b"SETINFO"), Some(b"LIB-VER")) => {
                    CommandKind::ConnectionState(StateSlot::LibVer)
                }
                (Some(b"NO-EVICT"), _) => CommandKind::ConnectionState(StateSlot::NoEvict),
                (Some(b"NO-TOUCH"), _) => CommandKind::ConnectionState(StateSlot::NoTouch),
                (Some(b"TRACKING"), _) => CommandKind::ConnectionState(StateSlot::Tracking),
                _ => CommandKind::Other,
            },
            b"AUTH" => CommandKind::ConnectionState(StateSlot::Auth),
            b"SELECT" => CommandKind::ConnectionState(StateSlot::Select),
            b"SCRIPT" if command.get_arg(0).as_deref() == Some(b"DEBUG") => {
                CommandKind::ConnectionState(StateSlot::ScriptDebug)
            }
            b"RESET" => CommandKind::Reset,
            _ => CommandKind::Other,
        }
    }
}

/// Generic command meant to be sent to the Redis Server
#[derive(Debug, Clone)]
pub struct Command {
    buffer: Bytes,
    kind: CommandKind,
    name_layout: (usize, usize),
    args_layout: ArgsLayout,
    #[doc(hidden)]
    #[cfg(test)]
    pub kill_connection_on_write: Arc<AtomicUsize>,
    #[doc(hidden)]
    #[cfg(test)]
    pub kill_connection_on_read: Arc<AtomicUsize>,
    #[cfg(test)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
    request_policy: Option<RequestPolicy>,
    response_policy: Option<ResponsePolicy>,
    key_step: u8,
    /// Whether the command only reads: a Cluster client may route it to a replica
    /// of the shard, which the server accepts only in `READONLY` mode.
    is_readonly: bool,
    /// A serialization error deferred from the builder, surfaced at send time.
    ///
    /// The fluent builder cannot return a `Result`, so a failing user
    /// `Serialize` impl is recorded here instead of panicking, and returned to
    /// the caller when the command reaches the network layer (see
    /// `Client::send_message`). Boxed to keep `Command` — memcpy'd several times
    /// per request — a single pointer wider in the common (no-error) case.
    serialization_error: Option<Box<crate::Error>>,
}

impl Command {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: Bytes,
        name_layout: (usize, usize),
        args_layout: ArgsLayout,
        #[cfg(test)] kill_connection_on_write: usize,
        #[cfg(test)] kill_connection_on_read: usize,
        #[cfg(test)] command_seq: usize,
        request_policy: Option<RequestPolicy>,
        response_policy: Option<ResponsePolicy>,
        key_step: u8,
        is_readonly: bool,
    ) -> Self {
        let mut this = Self {
            buffer,
            kind: CommandKind::Other,
            name_layout,
            args_layout,
            #[cfg(test)]
            kill_connection_on_write: Arc::new(kill_connection_on_write.into()),
            #[cfg(test)]
            kill_connection_on_read: Arc::new(kill_connection_on_read.into()),
            #[cfg(test)]
            command_seq,
            request_policy,
            response_policy,
            key_step,
            is_readonly,
            serialization_error: None,
        };

        this.kind = CommandKind::from(&this);
        this
    }

    /// Takes the serialization error deferred from the builder, if any.
    ///
    /// Returns `Some` at most once: the error is moved out so the send path can
    /// surface it to the caller and the command is left in its normal state.
    pub(crate) fn take_serialization_error(&mut self) -> Option<crate::Error> {
        self.serialization_error.take().map(|boxed| *boxed)
    }

    pub fn bytes(&self) -> &Bytes {
        &self.buffer
    }

    pub(crate) fn kind(&self) -> &CommandKind {
        &self.kind
    }

    /// Borrows the command name from the buffer.
    ///
    /// Returns a plain slice rather than an owned [`Bytes`] so that merely
    /// inspecting the name (e.g. classification) touches no atomic refcount.
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "invariant: `name_layout` was recorded by the builder while it \
                  wrote those very bytes into `buffer`; the two are produced \
                  together and never read off the wire, so the end offset lands \
                  inside the buffer."
    )]
    pub fn name(&self) -> &[u8] {
        let (start, len) = self.name_layout;
        &self.buffer[start..start + len]
    }

    /// The command name as an owned [`Bytes`], for the callers that must outlive
    /// the command — an error naming the command it belongs to, in particular.
    ///
    /// Costs one atomic increment and no copy: the command buffer is a plain
    /// `BytesMut::freeze()`, not a recycled one, so a slice of it may be held.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "invariant: same as `name`, `name_layout` is builder-produced \
                  and always lands inside `buffer`."
    )]
    pub(crate) fn name_bytes(&self) -> Bytes {
        let (start, len) = self.name_layout;
        self.buffer.slice(start..start + len)
    }

    pub fn get_arg(&self, index: usize) -> Option<Bytes> {
        let arg_layout = *self.args_layout.get(index)?;
        Some(self.buffer.slice(arg_layout.range()))
    }

    pub fn num_args(&self) -> usize {
        self.args_layout.len()
    }

    pub(crate) fn args_for_cluster(&self) -> impl Iterator<Item = (Bytes, bool, u16)> {
        self.args_layout
            .iter()
            .map(|al| (self.buffer.slice(al.range()), al.is_key(), al.slot))
    }

    pub fn args(&self) -> impl DoubleEndedIterator<Item = Bytes> {
        self.args_layout
            .iter()
            .map(|al| self.buffer.slice(al.range()))
    }

    pub fn keys(&self) -> impl DoubleEndedIterator<Item = Bytes> {
        self.args_layout
            .iter()
            .filter(|&al| al.is_key())
            .map(|al| self.buffer.slice(al.range()))
    }

    pub fn slots(&self) -> impl DoubleEndedIterator<Item = u16> {
        self.args_layout
            .iter()
            .filter(|&al| al.is_key())
            .map(|al| al.slot)
    }

    /// Computes the CRC16 hash slot of every key argument from its bytes.
    ///
    /// Runs on the *caller* thread and must be invoked only in Cluster mode,
    /// before the command is handed to the shared network thread (which then
    /// routes in O(1)) or its slots are inspected. Standalone clients skip this
    /// entirely and pay no CRC16 on their hot path.
    #[expect(
        clippy::indexing_slicing,
        reason = "invariant: an `ArgLayout` range is recorded by the builder as \
                  it appends the argument's bytes, so it always addresses this \
                  same buffer."
    )]
    pub(crate) fn compute_slots(&mut self) {
        for layout in &mut self.args_layout {
            if layout.is_key() {
                layout.slot = hash_slot(&self.buffer[layout.range()]);
            }
        }
    }

    pub fn request_policy(&self) -> Option<RequestPolicy> {
        self.request_policy.clone()
    }

    pub fn response_policy(&self) -> Option<ResponsePolicy> {
        self.response_policy.clone()
    }

    pub fn key_step(&self) -> usize {
        self.key_step as usize
    }

    /// Whether the command only reads, as declared by
    /// [`CommandBuilder::readonly`].
    pub fn is_readonly(&self) -> bool {
        self.is_readonly
    }

    #[cfg(test)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the fault-injection countdown is only decremented inside `> 0`. It \
                  is `cfg(test)` state: no shipped build reaches this."
    )]
    pub(crate) fn try_decrement_kill_connection_on_write(&self) -> bool {
        self.kill_connection_on_write
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                if current > 0 { Some(current - 1) } else { None }
            })
            .is_ok()
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.buffer == other.buffer
    }
}

impl Eq for Command {}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.buffer.hash(state);
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        String::from_utf8_lossy(self.name()).fmt(f)?;
        for arg in self.args() {
            f.write_char(' ')?;
            String::from_utf8_lossy(&arg).fmt(f)?;
        }

        Ok(())
    }
}

/// Builder for a [`Command`]
#[derive(Debug)]
pub struct CommandBuilder {
    /// The raw buffer containing the serialized arguments (in RESP format).
    /// It starts with `HEADROOM` bytes of zero-padding.
    pub(crate) buffer: BytesMut,
    /// Offset & Length of the command name
    pub(crate) name_layout: (usize, usize),
    /// An ephemeral index of argument positions (Start Offset, Length).
    ///
    /// This allows the `Client` to extract keys (for Cluster sharding) or
    /// channel names (for Pub/Sub) in O(1) time without re-parsing the buffer.
    /// This index is dropped when the command is sent to the network layer.
    pub(crate) args_layout: ArgsLayout,
    #[doc(hidden)]
    #[cfg(test)]
    pub kill_connection_on_write: usize,
    #[doc(hidden)]
    #[cfg(test)]
    pub kill_connection_on_read: usize,
    #[cfg(test)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
    pub(crate) request_policy: Option<RequestPolicy>,
    pub(crate) response_policy: Option<ResponsePolicy>,
    pub(crate) key_step: u8,
    pub(crate) is_readonly: bool,
    /// First serialization error encountered while building, deferred to send
    /// time so the fluent API stays panic-free (see [`Command`]).
    pub(crate) pending_error: Option<crate::Error>,
}

/// How many `step`-sized groups `count` arguments make, or `None` for a step of
/// zero — which names no group and would otherwise reach an integer division that
/// panics in release builds too.
///
/// The step comes from the caller of a public builder method, so it is validated
/// here rather than assumed.
#[inline(always)]
fn group_count(count: usize, step: usize) -> Option<usize> {
    count.checked_div(step)
}

impl CommandBuilder {
    /// The command name, for an error that names the command it belongs to.
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "invariant: `name_layout` was recorded by `new` while it wrote \
                  those very bytes into `buffer`, so the end offset lands inside \
                  the buffer."
    )]
    fn name(&self) -> &[u8] {
        let (start, len) = self.name_layout;
        &self.buffer[start..start + len]
    }

    /// Records the first deferred serialization error; later ones are ignored
    /// since the command is already doomed and the first is the most relevant.
    #[inline(always)]
    fn record_serialization_error(&mut self, error: crate::Error) {
        if self.pending_error.is_none() {
            self.pending_error = Some(error);
        }
    }

    /// Creates an new command.
    ///
    /// [`cmd`](crate::resp::cmd) function can be used as a shortcut.
    #[must_use]
    #[inline(always)]
    pub fn new(name: &[u8]) -> Self {
        let mut buffer = BytesMut::with_capacity(1024);

        // Reserve space for the header. These bytes will be overwritten later.
        buffer.put_bytes(0, HEADROOM_SIZE);

        // Write $NameLen\r\nName\r\n
        buffer.put_u8(b'$');
        let mut itoa_buf = itoa::Buffer::new();
        buffer.put_slice(itoa_buf.format(name.len()).as_bytes());
        buffer.put_slice(b"\r\n");
        let name_start = buffer.len();
        buffer.put_slice(name);
        buffer.put_slice(b"\r\n");

        Self {
            buffer,
            name_layout: (name_start, name.len()),
            args_layout: Default::default(),
            #[cfg(test)]
            kill_connection_on_write: 0,
            #[cfg(test)]
            kill_connection_on_read: 0,
            #[cfg(test)]
            command_seq: next_sequence_counter(),
            request_policy: None,
            response_policy: None,
            key_step: 0,
            is_readonly: false,
            pending_error: None,
        }
    }

    /// Builder function to add an argument to an existing command (uses Serde).
    ///
    /// An argument added this way is never considered a key for Cluster
    /// routing. Use [`key`](Self::key) for Redis keys.
    #[must_use]
    #[inline(always)]
    pub fn arg(mut self, arg: impl Serialize) -> Self {
        let result = {
            let mut serializer = ArgSerializer::new(&mut self.buffer, &mut self.args_layout);
            arg.serialize(&mut serializer)
        };
        if let Err(e) = result {
            self.record_serialization_error(e);
        }
        self
    }

    /// Builder function to add an argument to an existing command (uses Serde), only if a condition is `true`.
    #[must_use]
    #[inline(always)]
    pub fn arg_if(self, condition: bool, arg: impl Serialize) -> Self {
        if condition { self.arg(arg) } else { self }
    }

    /// Adds a collection or single argument prefixed by its element count.
    ///
    /// Uses a "Dry Run" (ArgCounter) to calculate the exact number of RESP
    /// arguments the collection produces (handling flattened maps/structs),
    /// then writes the count, then writes the elements.
    ///
    /// Zero Allocation strategy.
    #[must_use]
    #[inline(always)]
    pub fn arg_with_count(mut self, arg: impl Serialize) -> Self {
        // 1. Dry Run (CPU only, No Alloc)
        let mut counter = ArgCounter::default();
        if let Err(e) = arg.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }

        // 2. Write the count
        self = self.arg(counter.count);

        // 3. Write the elements
        self.arg_checking_count(arg, counter.count)
    }

    /// Adds a collection prefixed by the number of `step`-sized groups it
    /// contains, without marking anything as a routing key.
    ///
    /// The non-key counterpart of
    /// [`key_with_count_and_step`](Self::key_with_count_and_step), for commands
    /// whose grouped arguments live *inside* a key rather than being keys —
    /// `HSETEX key FIELDS numfields field value [field value ...]`, where the
    /// hash key is the only thing cluster routing cares about.
    ///
    /// Zero Allocation strategy.
    #[must_use]
    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`group_count` answered `Some`, so `step` is non-zero and the \
                  modulo in the assertion below cannot divide by zero."
    )]
    pub fn arg_with_count_and_step(mut self, arg: impl Serialize, step: usize) -> Self {
        // 1. Dry Run (CPU only, No Alloc) to get the total argument count.
        let mut counter = ArgCounter::default();
        if let Err(e) = arg.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }
        let Some(groups) = group_count(counter.count, step) else {
            self.record_serialization_error(Error::from(ClientError::InvalidArgumentGroupStep));
            return self;
        };
        debug_assert_eq!(
            0,
            counter.count % step,
            "arg_with_count_and_step: argument count {} is not a multiple of step {step}",
            counter.count
        );

        // 2. Write the group count, then the elements.
        self = self.arg(groups);
        self.arg_checking_count(arg, counter.count)
    }

    #[must_use]
    #[inline(always)]
    pub fn arg_labeled(mut self, label: &'static str, arg: impl Serialize) -> Self {
        // 1. Dry Run (CPU only, No Alloc)
        let mut counter = ArgCounter::default();
        if let Err(e) = arg.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }

        // 2. Conditionnally write the label + arg
        if counter.count != 0 {
            self.arg(label).arg(arg)
        } else {
            self
        }
    }

    /// Adds a labeled clause whose label is followed by the number of arguments
    /// the clause contains, as `SORTBY 2 field ASC` or `PARAMS 4 n1 v1 n2 v2`
    /// require. The count is derived from an `ArgCounter` dry run, so it cannot
    /// disagree with what is actually written; the label itself is not counted.
    ///
    /// `label` is serialized like any other argument, so a clause introduced by
    /// several tokens (`COMBINE RRF <count>`) can pass a tuple.
    ///
    /// The whole clause is skipped when it would contain no argument. A command
    /// that needs an explicit zero — RediSearch's `GROUPBY 0` means "group over
    /// everything" — must use `.arg(label).arg_with_count(arg)` instead.
    #[must_use]
    #[inline(always)]
    pub fn arg_counted(mut self, label: impl Serialize, arg: impl Serialize) -> Self {
        // 1. Dry Run (CPU only, No Alloc)
        let mut counter = ArgCounter::default();
        if let Err(e) = arg.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }
        if counter.count == 0 {
            return self;
        }

        // 2. Write the label and the count, then the elements.
        self = self.arg(label).arg(counter.count);
        self.arg_checking_count(arg, counter.count)
    }

    /// Appends `arg` and, in debug builds, checks that it produced exactly
    /// `expected` arguments.
    ///
    /// The count a command declares to the server always comes from an
    /// `ArgCounter` dry run, which makes it correct only insofar as `ArgCounter`
    /// and `ArgSerializer` agree. They are two separate implementations of the
    /// same traversal and have already drifted apart once, on empty-named
    /// struct fields. This turns every call site into a check of that
    /// agreement, at no cost in release builds.
    #[must_use]
    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`before` is the layout count taken before appending, and appending \
                  only ever grows it."
    )]
    fn arg_checking_count(mut self, arg: impl Serialize, expected: usize) -> Self {
        let before = self.args_layout.len();
        self = self.arg(arg);

        // A failed write stops part-way through, so the shortfall is expected.
        debug_assert!(
            self.pending_error.is_some() || self.args_layout.len() - before == expected,
            "the dry run counted {expected} arguments but {} were written",
            self.args_layout.len() - before
        );

        self
    }

    /// Adds a Key argument, marking it for Cluster routing. The CRC16 slot is
    /// computed later by `Command::compute_slots`, on the caller thread and
    /// only in Cluster mode.
    ///
    /// # Warning
    /// In Cluster mode, a command whose keys were added with
    /// [`arg`](Self::arg) carries no slot and is therefore sent to a **random
    /// node**. The server does not signal the mistake for single-key commands:
    /// it answers `MOVED`, the client refreshes its topology and retries — and
    /// picks a random node again. Multi-key commands such as `MSET` fail with
    /// `CROSSSLOT` instead. Both cases are silent misuses of the API, so every
    /// key must go through `key`:
    ///
    /// ```
    /// use rustis::resp::cmd;
    ///
    /// // routed on the slot of "key1"
    /// let routed = cmd("GET").key("key1");
    ///
    /// // sent to a random node
    /// let misrouted = cmd("GET").arg("key1");
    /// ```
    ///
    /// A multi-key command additionally requires all its keys to hash to the
    /// same slot; use hash tags (`{tag}key1`, `{tag}key2`) to guarantee it.
    ///
    /// # Arity
    /// The key must serialize to exactly one command argument. A collection of
    /// keys goes through [`keys`](Self::keys), a counted list through
    /// [`key_with_count`](Self::key_with_count). Anything else — `None`, an
    /// empty collection, a struct, a sequence — fails the command with
    /// [`InvalidKeyArity`](crate::ClientError::InvalidKeyArity) rather than
    /// reaching the server malformed and unrouted.
    #[must_use]
    #[inline(always)]
    pub fn key(self, key: impl Serialize) -> Self {
        self.key_of_arity(key, 1, "exactly one is required")
    }

    /// Adds every key of a collection, each marked for Cluster routing.
    ///
    /// The multi-key form of [`key`](Self::key), for commands whose grammar is a
    /// bare list of keys — `DEL`, `EXISTS`, `WATCH`, `SDIFF`. The collection may
    /// hold any number of keys but not none, which would send a command with no
    /// key and no slot.
    ///
    /// Commands that declare their key count to the server use
    /// [`key_with_count`](Self::key_with_count) instead, and may legally declare
    /// zero.
    #[must_use]
    #[inline(always)]
    pub fn keys(self, keys: impl Serialize) -> Self {
        self.key_of_arity(keys, usize::MAX, "at least one is required")
    }

    /// Adds `key` as one or more routing keys, failing the command unless it
    /// wrote between one and `at_most` command arguments.
    ///
    /// The count is what tells a key from a mistake. Arguments are
    /// `impl Serialize` so that any foreign type may be a key — the orphan rule
    /// leaves a marker trait unimplementable for one, by the crate that defines
    /// neither it nor the trait. That bound admits values writing no argument
    /// (`None`, an empty collection) and values writing several (a struct, a
    /// sequence), neither of which is a single key. The first is the damaging
    /// one: no argument means no slot, and a command with no slot is routed to a
    /// random node rather than refused.
    #[must_use]
    #[inline(always)]
    fn key_of_arity(mut self, key: impl Serialize, at_most: usize, expected: &'static str) -> Self {
        let old_len = self.args_layout.len();
        self = self.arg(key);
        let written = self.args_layout.len().saturating_sub(old_len);

        if written == 0 || written > at_most {
            // A failed write stops part-way through, so its shortfall is
            // expected and the failure is the more useful report of the two.
            if self.pending_error.is_none() {
                let command = String::from_utf8_lossy(self.name()).into_owned();
                self.record_serialization_error(Error::from(ClientError::InvalidKeyArity {
                    command,
                    written,
                    expected,
                }));
            }
            return self;
        }

        for layout in self.args_layout.iter_mut().skip(old_len) {
            layout.set_key();
        }

        self
    }

    /// Adds a collection or single key prefixed by its element count.
    ///
    /// Uses a "Dry Run" (ArgCounter) to calculate the exact number of RESP
    /// arguments the collection produces (handling flattened maps/structs),
    /// then writes the count, then writes the elements.
    ///
    /// Zero Allocation strategy.
    #[must_use]
    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`old_len` is a layout count, so skipping past it and the count \
                  argument written after it stays inside `usize`."
    )]
    pub fn key_with_count(mut self, keys: impl Serialize) -> Self {
        let old_len = self.args_layout.len();
        self = self.arg_with_count(keys);

        for layout in self.args_layout.iter_mut().skip(old_len + 1) {
            layout.flags |= ArgLayout::IS_KEY;
        }

        self
    }

    /// Serializes a collection and marks elements as keys based on a step.
    /// Example: for JSON.MSET, step is 3 (Key, Path, Value).
    #[must_use]
    #[inline(always)]
    pub fn key_with_step(mut self, args: impl Serialize, step: usize) -> Self {
        let old_len = self.args_layout.len();
        self = self.arg(args);

        for layout in self.args_layout.iter_mut().skip(old_len).step_by(step) {
            layout.flags |= ArgLayout::IS_KEY;
        }

        self
    }

    /// Serializes a collection prefixed by the number of `step`-sized groups it
    /// contains, then marks every `step`-th element as a key for cluster routing.
    ///
    /// Combines [`key_with_count`](Self::key_with_count) (leading count) and
    /// [`key_with_step`](Self::key_with_step) (stepped key flags) for commands
    /// such as MSETEX, whose grammar is `numkeys key value [key value ...]`.
    /// The emitted count is the number of groups (`total / step`), not the raw
    /// argument count.
    #[must_use]
    #[inline(always)]
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`group_count` answered `Some`, so `step` is non-zero and the \
                  modulo in the assertion below cannot divide by zero."
    )]
    pub fn key_with_count_and_step(mut self, args: impl Serialize, step: usize) -> Self {
        // 1. Dry Run (CPU only, No Alloc) to get the total argument count.
        let mut counter = ArgCounter::default();
        if let Err(e) = args.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }
        let Some(groups) = group_count(counter.count, step) else {
            self.record_serialization_error(Error::from(ClientError::InvalidArgumentGroupStep));
            return self;
        };
        debug_assert_eq!(
            0,
            counter.count % step,
            "key_with_count_and_step: argument count {} is not a multiple of step {step}",
            counter.count
        );

        // 2. Write the group count (number of key/value groups).
        self = self.arg(groups);

        // 3. Write the elements, marking every step-th one (after the count) as a key.
        let old_len = self.args_layout.len();
        self = self.arg_checking_count(args, counter.count);

        for layout in self.args_layout.iter_mut().skip(old_len).step_by(step) {
            layout.flags |= ArgLayout::IS_KEY;
        }

        self
    }

    #[cfg(test)]
    #[inline(always)]
    pub fn kill_connection_on_write(mut self, num_kills: usize) -> Self {
        self.kill_connection_on_write = num_kills;
        self
    }

    /// Arms the connection to be killed on the `num_reads`-th read attempt that
    /// follows this command being fed, before any response is delivered.
    ///
    /// The commands have already reached and been executed by the server (they
    /// were flushed), so this reproduces a disconnection occurring after
    /// server-side execution but before the client matches the responses.
    #[cfg(test)]
    #[inline(always)]
    pub fn kill_connection_on_read(mut self, num_reads: usize) -> Self {
        self.kill_connection_on_read = num_reads;
        self
    }

    #[inline(always)]
    pub fn cluster_info(
        mut self,
        request_policy: impl Into<Option<RequestPolicy>>,
        response_policy: impl Into<Option<ResponsePolicy>>,
        key_step: u8,
    ) -> Self {
        self.request_policy = request_policy.into();
        self.response_policy = response_policy.into();
        self.key_step = key_step;
        self
    }

    /// Declares the command as read-only, which is what allows a Cluster client
    /// configured with [`ReadPreference::PreferReplica`](crate::client::ReadPreference::PreferReplica)
    /// to route it to a replica of the shard.
    ///
    /// Mirrors the `readonly` flag the server reports in `COMMAND INFO`: a
    /// command that writes, blocks, or has a `STORE`-like variant must not
    /// declare it.
    #[inline(always)]
    pub fn readonly(mut self) -> Self {
        self.is_readonly = true;
        self
    }
}

impl From<CommandBuilder> for Command {
    /// Finalizes the command into a raw RESP frame.
    /// Fills the HEADROOM with the header and freezes the buffer.
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::cast_possible_truncation,
        reason = "invariant: every write below is bounded by `HEADROOM_SIZE`, \
                  which is sized to hold the longest `*<n>` header line and is \
                  reserved up front by `CommandBuilder::new`. The header therefore \
                  never fills the headroom, which is what makes `HEADROOM_SIZE - \
                  cursor.len()` and the shift of every layout by `start_pos` \
                  subtractions that cannot go below zero — and `start_pos`, \
                  bounded by that same headroom, exact as a `u32`. Nothing here is \
                  driven by input, and a fallback would have to emit a command \
                  with a truncated header — silent corruption in place of a \
                  crash. This exemption covers the finalizer only."
    )]
    fn from(mut command_builder: CommandBuilder) -> Self {
        // Stack buffer helpers
        fn write_u8(buf: &mut &mut [u8], val: u8) {
            buf[0] = val;
            *buf = &mut std::mem::take(buf)[1..];
        }

        fn write_slice(buf: &mut &mut [u8], val: &[u8]) {
            let len: usize = val.len();
            buf[..len].copy_from_slice(val);
            *buf = &mut std::mem::take(buf)[len..];
        }

        let total_args = 1 + command_builder.args_layout.len();

        // Temporary stack buffer for header formatting
        let mut header_buf = [0u8; HEADROOM_SIZE];
        let mut cursor = &mut header_buf[..];

        // Write *N\r\n
        write_u8(&mut cursor, b'*');
        let mut itoa_buf = itoa::Buffer::new();
        write_slice(&mut cursor, itoa_buf.format(total_args).as_bytes());
        write_slice(&mut cursor, b"\r\n");

        let header_len = HEADROOM_SIZE - cursor.len();
        let written_header = &header_buf[..header_len];

        // Copy header into HEADROOM
        let start_pos = HEADROOM_SIZE - header_len;
        command_builder.buffer[start_pos..HEADROOM_SIZE].copy_from_slice(written_header);

        let bytes = command_builder.buffer.freeze().slice(start_pos..);

        command_builder
            .args_layout
            .iter_mut()
            .for_each(|arg_layout| arg_layout.start -= start_pos as u32);

        let mut command = Command::new(
            bytes,
            (
                command_builder.name_layout.0 - start_pos,
                command_builder.name_layout.1,
            ),
            command_builder.args_layout,
            #[cfg(test)]
            command_builder.kill_connection_on_write,
            #[cfg(test)]
            command_builder.kill_connection_on_read,
            #[cfg(test)]
            command_builder.command_seq,
            command_builder.request_policy,
            command_builder.response_policy,
            command_builder.key_step,
            command_builder.is_readonly,
        );

        command.serialization_error = command_builder.pending_error.take().map(Box::new);
        command
    }
}

/// Implement hash_slot algorithm
/// see. https://redis.io/docs/latest/operate/oss_and_stack/reference/cluster-spec/#hash-tags
#[expect(
    clippy::arithmetic_side_effects,
    reason = "`s` is a `memchr` hit inside `key`, so stepping past the brace stays \
              an offset into a slice."
)]
pub(crate) fn hash_slot(mut key: &[u8]) -> u16 {
    // `{` found, then `}` after it, with a non-empty tag in between
    if let Some(s) = memchr(b'{', key)
        && let Some(after_brace) = key.get(s + 1..)
        && let Some(e) = memchr(b'}', after_brace)
        && e != 0
        && let Some(tag) = after_brace.get(..e)
    {
        key = tag;
    }

    crc16::State::<crc16::XMODEM>::calculate(key) % 16384
}

#[cfg(test)]
#[inline(always)]
pub(crate) fn next_sequence_counter() -> usize {
    COMMAND_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::indexing_slicing,
        reason = "test code: a panic is how a test reports failure"
    )]
    use crate::resp::{Command, cmd};

    #[test]
    fn command() {
        let command: Command = cmd("SET").arg("key").arg("value").into();
        println!("cmd: {command:?}");
        assert_eq!(b"SET", command.name());
        assert_eq!(Some(&b"key"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"value"[..]), command.get_arg(1).as_deref());
        assert_eq!(None, command.get_arg(2));

        let command: Command = cmd("EVAL").arg("return ARGV[1]").arg(0).arg("HELLO").into();
        println!("cmd: {command:?}");
        assert_eq!(b"EVAL", command.name());
        assert_eq!(Some(&b"return ARGV[1]"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"0"[..]), command.get_arg(1).as_deref());
        assert_eq!(Some(&b"HELLO"[..]), command.get_arg(2).as_deref());
    }

    struct FailingSerialize;
    impl serde::Serialize for FailingSerialize {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    #[test]
    fn arg_serialization_error_is_deferred_not_panicked() {
        let mut command: Command = cmd("PING").arg(FailingSerialize).into();
        assert!(matches!(
            command
                .take_serialization_error()
                .map(crate::Error::into_kind),
            Some(crate::ErrorKind::Client(
                crate::ClientError::SerdeSerialize(_)
            ))
        ));
        // The error is taken once, not re-yielded.
        assert!(command.take_serialization_error().is_none());
    }

    #[test]
    fn embedded_command_args_error_propagates_through_the_outer_builder() {
        let args = crate::resp::CommandArgsMut::default().arg(FailingSerialize);
        let mut command: Command = cmd("SORT").arg(args).into();
        assert!(command.take_serialization_error().is_some());
    }

    #[test]
    fn a_well_formed_command_carries_no_serialization_error() {
        let mut command: Command = cmd("SET").arg("key").arg("value").into();
        assert!(command.take_serialization_error().is_none());
    }

    #[test]
    fn arg_with_count_and_step_emits_the_group_count_not_the_argument_count() {
        let command: Command = cmd("HSETEX")
            .key("key")
            .arg("FIELDS")
            .arg_with_count_and_step(["f1", "v1", "f2", "v2"], 2)
            .into();
        assert_eq!(Some(&b"key"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"FIELDS"[..]), command.get_arg(1).as_deref());
        // Two field/value groups, not four arguments.
        assert_eq!(Some(&b"2"[..]), command.get_arg(2).as_deref());
        assert_eq!(Some(&b"f1"[..]), command.get_arg(3).as_deref());
        assert_eq!(Some(&b"v1"[..]), command.get_arg(4).as_deref());
        assert_eq!(Some(&b"f2"[..]), command.get_arg(5).as_deref());
        assert_eq!(Some(&b"v2"[..]), command.get_arg(6).as_deref());
        assert_eq!(None, command.get_arg(7));
    }

    #[test]
    fn arg_with_count_and_step_marks_no_element_as_a_key() {
        let command: Command = cmd("HSETEX")
            .key("key")
            .arg("FIELDS")
            .arg_with_count_and_step(["f1", "v1"], 2)
            .into();
        // The hash key is the only routing key; the fields inside it are not.
        assert_eq!(vec![&b"key"[..]], command.keys().collect::<Vec<_>>());
    }

    #[test]
    fn a_failing_arg_with_count_and_step_defers_instead_of_panicking() {
        let mut command: Command = cmd("HSETEX")
            .key("key")
            .arg("FIELDS")
            .arg_with_count_and_step(FailingSerialize, 2)
            .into();
        assert!(command.take_serialization_error().is_some());
    }

    #[test]
    fn a_zero_group_step_defers_instead_of_dividing_by_zero() {
        // The group count is `total / step`, so a zero step reaches an integer
        // division that panics in release builds as well as debug ones. These are
        // public builder methods: the step comes from the caller, and a caller
        // getting it wrong must fail the command, not the process.
        let mut command: Command = cmd("HSETEX")
            .key("key")
            .arg("FIELDS")
            .arg_with_count_and_step(["f1", "v1"], 0)
            .into();
        assert!(matches!(
            command
                .take_serialization_error()
                .map(crate::Error::into_kind),
            Some(crate::ErrorKind::Client(
                crate::ClientError::InvalidArgumentGroupStep
            ))
        ));

        let mut command: Command = cmd("MSETEX").key_with_count_and_step(["k", "v"], 0).into();
        assert!(matches!(
            command
                .take_serialization_error()
                .map(crate::Error::into_kind),
            Some(crate::ErrorKind::Client(
                crate::ClientError::InvalidArgumentGroupStep
            ))
        ));
    }

    /// The arity a key argument reported, or `None` when the command carries no
    /// error at all.
    fn key_arity_error(mut command: Command) -> Option<usize> {
        match command
            .take_serialization_error()
            .map(crate::Error::into_kind)
        {
            Some(crate::ErrorKind::Client(crate::ClientError::InvalidKeyArity {
                written, ..
            })) => Some(written),
            Some(other) => panic!("expected an arity error, got {other:?}"),
            None => None,
        }
    }

    /// `None` writes no argument at all, so the command reaches the server one
    /// argument short — and, worse, carries no hash slot, which routes it to a
    /// random node instead of the one that owns the key.
    #[test]
    fn a_key_serializing_to_no_argument_fails_the_command() {
        let command: Command = cmd("GET").key(None::<&str>).into();
        assert_eq!(Some(0), key_arity_error(command));
    }

    /// A sequence writes one argument per element, and a struct writes two —
    /// name and value — per field, which is what makes `HSET` take a struct.
    /// Nothing in the `impl Serialize` bound says a key is a single value, so
    /// the count is what tells them apart.
    #[test]
    fn a_key_serializing_to_several_arguments_fails_the_command() {
        let command: Command = cmd("GET").key(["a", "b"]).into();
        assert_eq!(Some(2), key_arity_error(command));

        #[derive(serde::Serialize)]
        struct CompositeKey {
            tenant: &'static str,
            id: u64,
        }
        let command: Command = cmd("GET")
            .key(CompositeKey {
                tenant: "acme",
                id: 42,
            })
            .into();
        assert_eq!(Some(4), key_arity_error(command), "two fields, four args");
    }

    /// The point of checking the count rather than the type: a foreign type the
    /// crate has never heard of is a valid key as soon as it writes one
    /// argument. A marker trait could not accept it — the orphan rule leaves
    /// neither side able to write the impl.
    #[test]
    fn a_foreign_type_writing_one_argument_is_a_valid_key() {
        /// Serializes as a single string, the way `uuid::Uuid` does.
        struct ForeignId;
        impl serde::Serialize for ForeignId {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str("67e55044-10b1-426f-9247-bb680e5fe0c8")
            }
        }

        let command: Command = cmd("GET").key(ForeignId).into();
        assert_eq!(
            vec![&b"67e55044-10b1-426f-9247-bb680e5fe0c8"[..]],
            command.keys().collect::<Vec<_>>()
        );
        assert_eq!(None, key_arity_error(command));
    }

    /// A key that failed to serialize keeps its own error: the shortfall is a
    /// consequence of the failure, so reporting the count would hide the cause.
    #[test]
    fn a_failed_key_serialization_keeps_its_own_error() {
        let mut command: Command = cmd("GET").key(FailingSerialize).into();
        assert!(matches!(
            command
                .take_serialization_error()
                .map(crate::Error::into_kind),
            Some(crate::ErrorKind::Client(
                crate::ClientError::SerdeSerialize(_)
            ))
        ));
    }

    /// A multi-key command takes a collection, so its keys are added through
    /// [`CommandBuilder::keys`], which allows any number of them — but not none,
    /// which is the same slotless command as above.
    #[test]
    fn a_collection_of_keys_is_accepted_and_every_key_is_routed() {
        let command: Command = cmd("DEL").keys(["k1", "k2", "k3"]).into();
        assert_eq!(
            vec![&b"k1"[..], &b"k2"[..], &b"k3"[..]],
            command.keys().collect::<Vec<_>>()
        );
        assert_eq!(None, key_arity_error(command));
    }

    #[test]
    fn an_empty_collection_of_keys_fails_the_command() {
        let command: Command = cmd("DEL").keys(Vec::<&str>::new()).into();
        assert_eq!(Some(0), key_arity_error(command));
    }

    /// `EVAL` and friends declare their key count to the server, and declaring
    /// zero is legal: the script takes its arguments from `ARGV` alone. The
    /// counted forms therefore check nothing.
    #[test]
    fn a_counted_key_list_may_legally_be_empty() {
        let command: Command = cmd("EVAL")
            .arg("return 1")
            .key_with_count(Vec::<&str>::new())
            .into();
        assert_eq!(Some(&b"0"[..]), command.get_arg(1).as_deref());
        assert_eq!(0, command.keys().count());
        assert_eq!(None, key_arity_error(command));
    }
}
