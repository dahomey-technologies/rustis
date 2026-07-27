use crate::{
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandKind {
    Other,
    Unsbuscribe(SubscriptionType),
    ClientReply(ClientReplyMode),
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

impl ArgLayout {
    /// Flag indicating that this argument is a Redis key.
    const IS_KEY: u16 = 1 << 0;

    #[inline(always)]
    pub fn arg(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start as u32,
            len: range.end as u32 - range.start as u32,
            slot: 0,
            flags: 0,
        }
    }

    #[inline(always)]
    pub fn key(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start as u32,
            len: range.end as u32 - range.start as u32,
            slot: 0,
            flags: Self::IS_KEY,
        }
    }

    #[inline(always)]
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.start as usize + self.len as usize
    }

    #[inline(always)]
    pub fn is_key(&self) -> bool {
        self.flags & Self::IS_KEY != 0
    }

    #[inline(always)]
    pub fn set_key(&mut self) {
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
                _ => CommandKind::Other,
            },
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
        reason = "invariant: `name_layout` was recorded by the builder while it \
                  wrote those very bytes into `buffer`; the two are produced \
                  together and never read off the wire."
    )]
    pub fn name(&self) -> &[u8] {
        let (start, len) = self.name_layout;
        &self.buffer[start..start + len]
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

    #[cfg(test)]
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
    /// First serialization error encountered while building, deferred to send
    /// time so the fluent API stays panic-free (see [`Command`]).
    pub(crate) pending_error: Option<crate::Error>,
}

impl CommandBuilder {
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
            pending_error: None,
        }
    }

    /// Builder function to add an argument to an existing command (uses Serde).
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
    pub fn arg_with_count_and_step(mut self, arg: impl Serialize, step: usize) -> Self {
        // 1. Dry Run (CPU only, No Alloc) to get the total argument count.
        let mut counter = ArgCounter::default();
        if let Err(e) = arg.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }
        debug_assert!(
            counter.count % step == 0,
            "arg_with_count_and_step: argument count {} is not a multiple of step {step}",
            counter.count
        );

        // 2. Write the group count, then the elements.
        self = self.arg(counter.count / step);
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
    /// computed later by [`Command::compute_slots`], on the caller thread and
    /// only in Cluster mode.
    #[must_use]
    #[inline(always)]
    pub fn key(mut self, key: impl Serialize) -> Self {
        let old_len = self.args_layout.len();
        self = self.arg(key);

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
    pub fn key_with_count_and_step(mut self, args: impl Serialize, step: usize) -> Self {
        // 1. Dry Run (CPU only, No Alloc) to get the total argument count.
        let mut counter = ArgCounter::default();
        if let Err(e) = args.serialize(&mut counter) {
            self.record_serialization_error(e);
            return self;
        }
        debug_assert!(
            counter.count % step == 0,
            "key_with_count_and_step: argument count {} is not a multiple of step {step}",
            counter.count
        );

        // 2. Write the group count (number of key/value groups).
        self = self.arg(counter.count / step);

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
}

impl From<CommandBuilder> for Command {
    /// Finalizes the command into a raw RESP frame.
    /// Fills the HEADROOM with the header and freezes the buffer.
    #[expect(
        clippy::indexing_slicing,
        reason = "invariant: every write below is bounded by `HEADROOM_SIZE`, \
                  which is sized to hold the longest `*<n>` header line and is \
                  reserved up front by `CommandBuilder::new`. Nothing here is \
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
        );

        command.serialization_error = command_builder.pending_error.take().map(Box::new);
        command
    }
}

/// Implement hash_slot algorithm
/// see. https://redis.io/docs/latest/operate/oss_and_stack/reference/cluster-spec/#hash-tags
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
            command.take_serialization_error(),
            Some(crate::Error::Client(crate::ClientError::SerdeSerialize(_)))
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
}
