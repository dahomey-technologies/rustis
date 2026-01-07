use crate::resp::{ArgCounter, ArgSerializer, CommandArgsIterator};
use bytes::{BufMut, Bytes, BytesMut};
use serde::Serialize;
use smallvec::SmallVec;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicUsize, Ordering};
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

/// Generic command meant to be sent to the Redis Server
#[derive(Debug, Clone, Eq)]
pub struct Command {
    buffer: Bytes,
    name_layout: (usize, usize),
    args_layout: SmallVec<[(usize, usize); 10]>,
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub kill_connection_on_write: usize,
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
}

impl Command {
    pub fn new(
        buffer: Bytes,
        name_layout: (usize, usize),
        args_layout: SmallVec<[(usize, usize); 10]>,
        #[cfg(debug_assertions)] kill_connection_on_write: usize,
        #[cfg(debug_assertions)] command_seq: usize,
    ) -> Self {
        Self {
            buffer,
            name_layout,
            args_layout,
            #[cfg(debug_assertions)]
            kill_connection_on_write,
            #[cfg(debug_assertions)]
            command_seq,
        }
    }

    pub fn get_bytes(&self) -> &Bytes {
        &self.buffer
    }

    pub fn get_name(&self) -> Bytes {
        let (start, len) = self.name_layout;
        self.buffer.slice(start..start + len)
    }

    pub fn get_arg(&self, index: usize) -> Option<Bytes> {
        let (start, len) = *self.args_layout.get(index)?;
        Some(self.buffer.slice(start..start + len))
    }

    pub fn num_args(&self) -> usize {
        self.args_layout.len()
    }

    pub fn args<'a>(&'a self) -> CommandArgsIterator<'a> {
        CommandArgsIterator {
            buffer: self.buffer.clone(),
            layout_iter: self.args_layout.iter(),
        }
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.buffer == other.buffer
    }
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.buffer.hash(state);
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        String::from_utf8_lossy(&self.get_name()).fmt(f)?;
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
    pub(crate) args_layout: SmallVec<[(usize, usize); 10]>,
    #[doc(hidden)]
    #[cfg(debug_assertions)]
    pub kill_connection_on_write: usize,
    #[cfg(debug_assertions)]
    #[allow(unused)]
    pub(crate) command_seq: usize,
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
            command_seq: COMMAND_SEQUENCE_COUNTER.fetch_add(1, Ordering::SeqCst),
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
    #[inline]
    pub fn arg_with_count(mut self, arg: impl Serialize) -> Self {
        // 1. Dry Run (CPU only, No Alloc)
        let mut counter = ArgCounter::default();
        arg.serialize(&mut counter).expect("Arg counting failed");

        // 2. Write the count
        self = self.arg(counter.count);

        // 3. Write the elements
        self.arg(arg)
    }

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

    #[cfg(debug_assertions)]
    #[inline]
    pub fn kill_connection_on_write(mut self, num_kills: usize) -> Self {
        self.kill_connection_on_write = num_kills;
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
            .for_each(|(s, _l)| *s -= start_pos);

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
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::resp::{Command, cmd};

    #[test]
    fn command() {
        let command: Command = cmd("SET").arg("key").arg("value").into();
        println!("cmd: {command:?}");
        assert_eq!(b"SET", command.get_name().as_ref());
        assert_eq!(Some(&b"key"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"value"[..]), command.get_arg(1).as_deref());
        assert_eq!(None, command.get_arg(2));

        let command: Command = cmd("EVAL").arg("return ARGV[1]").arg(0).arg("HELLO").into();
        println!("cmd: {command:?}");
        assert_eq!(b"EVAL", command.get_name().as_ref());
        assert_eq!(Some(&b"return ARGV[1]"[..]), command.get_arg(0).as_deref());
        assert_eq!(Some(&b"0"[..]), command.get_arg(1).as_deref());
        assert_eq!(Some(&b"HELLO"[..]), command.get_arg(2).as_deref());
    }
}
