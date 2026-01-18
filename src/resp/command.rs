use crate::{
    commands::{RequestPolicy, ResponsePolicy},
    resp::{ArgCounter, ArgSerializer},
};
use bytes::{BufMut, Bytes, BytesMut};
use memchr::memchr;
use serde::Serialize;
use smallvec::SmallVec;
#[cfg(debug_assertions)]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
};

#[cfg(debug_assertions)]
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
/// This structure is packed into 128 bits (16 bytes) to minimize its footprint.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub(crate) struct ArgLayout {
    /// The starting position of the argument data within the command's internal buffer.
    pub start: u64,

    /// The length of the argument data in bytes.
    /// Redis limits bulk strings to 512MB, so `u32` is more than sufficient.
    pub len: u32,

    /// The CRC16 hash slot (0-16383) calculated for this argument.
    /// This is pre-calculated by the client thread to allow O(1) routing
    /// in Cluster mode without further CPU overhead in the network thread.
    pub slot: u16,

    /// Bitwise flags for argument properties.
    /// - Bit 0: `IS_KEY` (indicates if the argument is a Redis key for routing).
    /// - Remaining bits: Reserved for future use (e.g., compression, encryption).
    pub flags: u16,
}

impl ArgLayout {
    /// Flag indicating that this argument is a Redis key.
    const IS_KEY: u16 = 1 << 0;

    #[inline(always)]
    pub fn arg(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start as u64,
            len: range.end as u32 - range.start as u32,
            slot: 0,
            flags: 0,
        }
    }

    #[inline(always)]
    pub fn key(range: std::ops::Range<usize>, slot: u16) -> Self {
        Self {
            start: range.start as u64,
            len: range.end as u32 - range.start as u32,
            slot,
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
        match command.name().as_ref() {
            b"UNSUBSCRIBE" => CommandKind::Unsbuscribe(SubscriptionType::Channel),
            b"PUNSUBSCRIBE" => CommandKind::Unsbuscribe(SubscriptionType::Pattern),
            b"SUNSUBSCRIBE" => CommandKind::Unsbuscribe(SubscriptionType::ShardChannel),
            b"CLIENT" => match (command.get_arg(0).as_deref(), command.get_arg(1).as_deref()) {
                (Some(b"REPLY"), Some(b"ON")) => CommandKind::ClientReply(ClientReplyMode::On),
                (Some(b"REPLY"), Some(b"OFF")) => CommandKind::ClientReply(ClientReplyMode::Off),
                (Some(b"REPLY"), Some(b"SKIPP")) => CommandKind::ClientReply(ClientReplyMode::Skip),
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
    args_layout: SmallVec<[ArgLayout; 10]>,
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub kill_connection_on_write: Arc<AtomicUsize>,
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
    request_policy: Option<RequestPolicy>,
    response_policy: Option<ResponsePolicy>,
    key_step: u8,
}

impl Command {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        buffer: Bytes,
        name_layout: (usize, usize),
        args_layout: SmallVec<[ArgLayout; 10]>,
        #[cfg(debug_assertions)] kill_connection_on_write: usize,
        #[cfg(debug_assertions)] command_seq: usize,
        request_policy: Option<RequestPolicy>,
        response_policy: Option<ResponsePolicy>,
        key_step: u8,
    ) -> Self {
        let mut this = Self {
            buffer,
            kind: CommandKind::Other,
            name_layout,
            args_layout,
            #[cfg(debug_assertions)]
            kill_connection_on_write: Arc::new(kill_connection_on_write.into()),
            #[cfg(debug_assertions)]
            command_seq,
            request_policy,
            response_policy,
            key_step,
        };

        this.kind = CommandKind::from(&this);
        this
    }

    pub fn bytes(&self) -> &Bytes {
        &self.buffer
    }

    pub(crate) fn kind(&self) -> &CommandKind {
        &self.kind
    }

    pub fn name(&self) -> Bytes {
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

    pub fn request_policy(&self) -> Option<RequestPolicy> {
        self.request_policy.clone()
    }

    pub fn response_policy(&self) -> Option<ResponsePolicy> {
        self.response_policy.clone()
    }

    pub fn key_step(&self) -> usize {
        self.key_step as usize
    }

    #[cfg(debug_assertions)]
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
        String::from_utf8_lossy(&self.name()).fmt(f)?;
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
    pub(crate) args_layout: SmallVec<[ArgLayout; 10]>,
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub kill_connection_on_write: usize,
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
    pub(crate) request_policy: Option<RequestPolicy>,
    pub(crate) response_policy: Option<ResponsePolicy>,
    pub(crate) key_step: u8,
}

impl CommandBuilder {
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
            #[cfg(debug_assertions)]
            kill_connection_on_write: 0,
            #[cfg(debug_assertions)]
            command_seq: next_sequence_counter(),
            request_policy: None,
            response_policy: None,
            key_step: 0,
        }
    }

    /// Builder function to add an argument to an existing command (uses Serde).
    #[must_use]
    #[inline(always)]
    pub fn arg(mut self, arg: impl Serialize) -> Self {
        let mut serializer = ArgSerializer::new(&mut self.buffer, &mut self.args_layout);
        arg.serialize(&mut serializer)
            .expect("Arg serialization failed");
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
        arg.serialize(&mut counter).expect("Arg counting failed");

        // 2. Write the count
        self = self.arg(counter.count);

        // 3. Write the elements
        self.arg(arg)
    }

    #[must_use]
    #[inline(always)]
    pub fn arg_labeled(self, label: &'static str, arg: impl Serialize) -> Self {
        // 1. Dry Run (CPU only, No Alloc)
        let mut counter = ArgCounter::default();
        arg.serialize(&mut counter).expect("Arg counting failed");

        // 2. Conditionnally write the label + arg
        if counter.count != 0 {
            self.arg(label).arg(arg)
        } else {
            self
        }
    }

    /// Adds a Key argument and calculates its CRC16 slot immediately.
    #[must_use]
    #[inline(always)]
    pub fn key(mut self, key: impl Serialize) -> Self {
        let old_len = self.args_layout.len();
        self = self.arg(key);
        let new_len = self.args_layout.len();

        for layout in &mut self.args_layout[old_len..new_len] {
            layout.set_key();
            let key_bytes = &self.buffer[layout.range()];
            layout.slot = hash_slot(key_bytes);
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
        let new_len = self.args_layout.len();

        for layout in &mut self.args_layout[old_len + 1..new_len] {
            layout.flags |= ArgLayout::IS_KEY;
            let key_bytes = &self.buffer[layout.range()];
            layout.slot = hash_slot(key_bytes);
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
        let new_len = self.args_layout.len();

        for layout in &mut self.args_layout[old_len..new_len].iter_mut().step_by(step) {
            layout.flags |= ArgLayout::IS_KEY;
            let key_bytes = &self.buffer[layout.range()];
            layout.slot = hash_slot(key_bytes);
        }

        self
    }

    #[cfg(debug_assertions)]
    #[inline(always)]
    pub fn kill_connection_on_write(mut self, num_kills: usize) -> Self {
        self.kill_connection_on_write = num_kills;
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
            .for_each(|arg_layout| arg_layout.start -= start_pos as u64);

        Command::new(
            bytes,
            (
                command_builder.name_layout.0 - start_pos,
                command_builder.name_layout.1,
            ),
            command_builder.args_layout,
            #[cfg(debug_assertions)]
            command_builder.kill_connection_on_write,
            #[cfg(debug_assertions)]
            command_builder.command_seq,
            command_builder.request_policy,
            command_builder.response_policy,
            command_builder.key_step,
        )
    }
}

/// Implement hash_slot algorithm
/// see. https://redis.io/docs/latest/operate/oss_and_stack/reference/cluster-spec/#hash-tags
pub(crate) fn hash_slot(mut key: &[u8]) -> u16 {
    // { found
    if let Some(s) = memchr(b'{', key) {
        // } found
        if let Some(e) = memchr(b'}', &key[s + 1..]) {
            // hash tag non empty
            if e != 0 {
                key = &key[s + 1..s + 1 + e];
            }
        }
    }

    crc16::State::<crc16::XMODEM>::calculate(key) % 16384
}

#[cfg(debug_assertions)]
#[inline(always)]
pub(crate) fn next_sequence_counter() -> usize {
    COMMAND_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use crate::resp::{Command, cmd};

    #[test]
    fn command() {
        let command: Command = cmd("SET").arg("key").arg("value").into();
        println!("cmd: {command:?}");
        assert_eq!(b"SET", command.name().as_ref());
        assert_eq!(Some(&b"key"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"value"[..]), command.get_arg(1).as_deref());
        assert_eq!(None, command.get_arg(2));

        let command: Command = cmd("EVAL").arg("return ARGV[1]").arg(0).arg("HELLO").into();
        println!("cmd: {command:?}");
        assert_eq!(b"EVAL", command.name().as_ref());
        assert_eq!(Some(&b"return ARGV[1]"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"0"[..]), command.get_arg(1).as_deref());
        assert_eq!(Some(&b"HELLO"[..]), command.get_arg(2).as_deref());
    }
}
