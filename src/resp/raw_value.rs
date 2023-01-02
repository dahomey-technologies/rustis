use std::ops::Range;

pub enum ValueSequenceType {
    Array,
    Set,
    Push
}

#[derive(Debug, Clone)]
pub enum RawValue {
    SimpleString(Range<usize>),
    Error(Range<usize>),
    Integer(Range<usize>),
    BulkString(Range<usize>),
    Array(usize),
    Map(usize),
    Set(usize),
    Double(Range<usize>),
    Nil,
    Bool(Range<usize>),
    VerbatimString(Range<usize>),
    BlobError(Range<usize>),
    Push(usize)
}

impl RawValue {
    #[inline]
    pub fn new_sequence(sequence_type: ValueSequenceType, len: usize) -> Self {
        match sequence_type {
            ValueSequenceType::Array => Self::Array(len),
            ValueSequenceType::Set => Self::Set(len),
            ValueSequenceType::Push => Self::Push(len),
        }
    }
}
