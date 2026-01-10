use crate::resp::ArgSerializer;
use bytes::{Bytes, BytesMut};
use serde::{Serialize, ser::SerializeSeq};
use smallvec::SmallVec;

/// A specialized buffer for Redis command arguments.
///
/// This structure acts as a "RESP Writer". It holds the raw bytes of the arguments
/// and maintains a layout index to allow random access to arguments before the
/// command is finalized.
pub struct CommandArgsMut {
    /// The raw buffer containing the serialized arguments (in RESP format).
    pub(crate) buffer: BytesMut,
    /// An ephemeral index of argument positions (Start Offset, Length).
    ///
    /// This allows the `Client` to extract keys (for Cluster sharding) or
    /// channel names (for Pub/Sub) in O(1) time without re-parsing the buffer.
    /// This index is dropped when the command is sent to the network layer.
    pub(crate) args_layout: SmallVec<[(usize, usize); 10]>,
}

impl Default for CommandArgsMut {
    fn default() -> Self {
        Self {
            buffer: BytesMut::with_capacity(1024),
            args_layout: Default::default(),
        }
    }
}

impl CommandArgsMut {
    #[inline(always)]
    pub fn arg(mut self, arg: impl Serialize) -> Self {
        let mut serializer = ArgSerializer::new(&mut self.buffer, &mut self.args_layout);
        arg.serialize(&mut serializer)
            .expect("Arg serialization failed");
        self
    }

    /// Returns the number of arguments currently written.
    #[inline]
    pub fn len(&self) -> usize {
        self.args_layout.len()
    }

    /// Returns `true` if there is no argument
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.args_layout.is_empty()
    }

    #[inline]
    pub fn freeze(self) -> CommandArgs {
        CommandArgs {
            buffer: self.buffer.freeze(),
            args_layout: self.args_layout,
        }
    }
}

impl Serialize for CommandArgsMut {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        struct RawBytes<'a>(&'a [u8]);

        impl<'a> Serialize for RawBytes<'a> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_bytes(self.0)
            }
        }

        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        for (start, len) in &self.args_layout {
            let arg_bytes = &self.buffer[*start..*start + *len];
            seq.serialize_element(&RawBytes(arg_bytes))?;
        }

        seq.end()
    }
}

/// A specialized buffer for Redis command arguments.
///
/// This structure acts as a "RESP Writer". It holds the raw bytes of the arguments
/// and maintains a layout index to allow random access to arguments before the
/// command is finalized.
#[derive(Default)]
pub struct CommandArgs {
    /// The raw buffer containing the serialized arguments (in RESP format).
    pub(crate) buffer: Bytes,
    /// An ephemeral index of argument positions (Start Offset, Length).
    ///
    /// This allows the `Client` to extract keys (for Cluster sharding) or
    /// channel names (for Pub/Sub) in O(1) time without re-parsing the buffer.
    /// This index is dropped when the command is sent to the network layer.
    pub(crate) args_layout: SmallVec<[(usize, usize); 10]>,
}

impl CommandArgs {
    /// Returns the number of arguments currently written.
    #[inline]
    pub fn len(&self) -> usize {
        self.args_layout.len()
    }

    /// Returns `true` if there is no argument
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.args_layout.is_empty()
    }

    pub fn iter(&self) -> CommandArgsIterator<'_> {
        self.into_iter()
    }
}

impl Serialize for CommandArgs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;

        for arg in self {
            seq.serialize_element(&arg)?;
        }

        seq.end()
    }
}

impl<'a> IntoIterator for &'a CommandArgs {
    type Item = Bytes;
    type IntoIter = CommandArgsIterator<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        CommandArgsIterator {
            buffer: self.buffer.clone(),
            layout_iter: self.args_layout.iter(),
        }
    }
}

/// [`CommandArgs`] iterator
pub struct CommandArgsIterator<'a> {
    pub(crate) buffer: Bytes,
    pub(crate) layout_iter: std::slice::Iter<'a, (usize, usize)>,
}

impl<'a> Iterator for CommandArgsIterator<'a> {
    type Item = Bytes;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let (start, len) = self.layout_iter.next()?;
        Some(self.buffer.slice(*start..*start + *len))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.layout_iter.size_hint()
    }
}

impl<'a> DoubleEndedIterator for CommandArgsIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (start, len) = self.layout_iter.next_back()?;
        Some(self.buffer.slice(*start..*start + *len))
    }
}

impl<'a> ExactSizeIterator for CommandArgsIterator<'a> {
    fn len(&self) -> usize {
        self.layout_iter.len()
    }
}
