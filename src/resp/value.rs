use crate::{RedisError, Result};
use serde::de::DeserializeOwned;
use std::fmt::{self, Display, Formatter, Write};

/// Generic Redis Object Model
///
/// This enum is a direct mapping to [`Redis serialization protocol`](https://redis.io/docs/latest/develop/reference/protocol-spec) (RESP)
#[derive(Default)]
pub enum Value {
    /// [RESP Simple String](https://redis.io/docs/latest/develop/reference/protocol-spec/#simple-strings)
    SimpleString(String),
    /// [RESP Integer](https://redis.io/docs/latest/develop/reference/protocol-spec/#integers)
    Integer(i64),
    /// [RESP Double](https://redis.io/docs/latest/develop/reference/protocol-spec/#doubles)
    ///
    /// Equality on this variant is total, as `Value` is `Eq`: all NaNs are
    /// equal to each other — so a `,nan` reply equals itself — and `-0.0`
    /// equals `0.0`. Both depart from IEEE-754, which has no reflexive NaN.
    Double(f64),
    /// [RESP Bulk String](https://redis.io/docs/latest/develop/reference/protocol-spec/#bulk-strings)
    BulkString(Vec<u8>),
    /// [RESP Boolean](https://redis.io/docs/latest/develop/reference/protocol-spec/#booleans)
    Boolean(bool),
    /// [RESP Array](https://redis.io/docs/latest/develop/reference/protocol-spec/#arrays)
    Array(Vec<Value>),
    /// [RESP Map](https://redis.io/docs/latest/develop/reference/protocol-spec/#maps)
    ///
    /// The entries are in the order the server sent them, and a field the
    /// server repeats appears twice. A `HashMap` would lose both, and `Value`
    /// is the fallback a caller reaches for precisely when it does not model
    /// the reply shape: it must hand back the reply itself. Callers that want
    /// map semantics deserialize into their own `HashMap`.
    Map(Vec<(Value, Value)>),
    /// [RESP Set](https://redis.io/docs/latest/develop/reference/protocol-spec/#sets)
    Set(Vec<Value>),
    /// [RESP Push](https://redis.io/docs/latest/develop/reference/protocol-spec/#pushes)
    Push(Vec<Value>),
    /// [RESP Error](https://redis.io/docs/latest/develop/reference/protocol-spec/#simple-errors)
    Error(RedisError),
    /// [RESP Null](https://redis.io/docs/latest/develop/reference/protocol-spec/#nulls)
    #[default]
    Null,
}

impl Value {
    /// A [`Value`](crate::resp::Value) to user type conversion that consumes the input value.
    ///
    /// # Errors
    /// Any parsing error ([`ErrorKind::Client`](crate::ErrorKind::Client)) due to incompatibility between Value variant and taget type
    #[inline]
    pub fn into<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(&self)
    }

    /// The entries of a [`Value::Map`], in the order the server sent them,
    /// [`None`] for any other variant.
    #[inline]
    #[must_use]
    pub fn as_map(&self) -> Option<&[(Value, Value)]> {
        match self {
            Value::Map(entries) => Some(entries),
            _ => None,
        }
    }

    /// The value of the first entry of a [`Value::Map`] whose field equals
    /// `key`, [`None`] if there is none or if this is not a map.
    ///
    /// The scan is linear, because the entries are a sequence: a RESP3 map may
    /// repeat a field, and this answers the first of them. Callers doing many
    /// lookups over a large reply should deserialize into their own map.
    #[inline]
    #[must_use]
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.as_map()?
            .iter()
            .find(|(field, _)| field == key)
            .map(|(_, value)| value)
    }
}

/// Canonical bit pattern a [`Value::Double`] is compared on.
///
/// `Value` asserts `Eq`, which demands a reflexive equality, so the IEEE-754
/// rules are not usable as they stand: every NaN collapses onto a single
/// pattern, and the two zeros — equal under `==` — onto the positive one.
fn canonical_double_bits(d: f64) -> u64 {
    if d.is_nan() {
        f64::NAN.to_bits()
    } else if d == 0.0 {
        0.0f64.to_bits()
    } else {
        d.to_bits()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SimpleString(l0), Self::SimpleString(r0)) => l0 == r0,
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Double(l0), Self::Double(r0)) => {
                canonical_double_bits(*l0) == canonical_double_bits(*r0)
            }
            (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
            (Self::BulkString(l0), Self::BulkString(r0)) => l0 == r0,
            (Self::Array(l0), Self::Array(r0)) => l0 == r0,
            (Self::Map(l0), Self::Map(r0)) => l0 == r0,
            (Self::Set(l0), Self::Set(r0)) => l0 == r0,
            (Self::Push(l0), Self::Push(r0)) => l0 == r0,
            (Self::Error(l0), Self::Error(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            Value::SimpleString(s) => s.fmt(f),
            Value::Integer(i) => i.fmt(f),
            Value::Double(d) => d.fmt(f),
            Value::BulkString(s) => String::from_utf8_lossy(s).fmt(f),
            Value::Boolean(b) => b.fmt(f),
            Value::Array(v) => {
                f.write_char('[')?;
                let mut first = true;
                for value in v {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    value.fmt(f)?;
                }
                f.write_char(']')
            }
            Value::Map(entries) => {
                f.write_char('{')?;
                let mut first = true;
                for (key, value) in entries {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    key.fmt(f)?;
                    f.write_str(": ")?;
                    value.fmt(f)?;
                }
                f.write_char('}')
            }
            Value::Set(v) => {
                f.write_char('[')?;
                let mut first = true;
                for value in v {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    value.fmt(f)?;
                }
                f.write_char(']')
            }
            Value::Push(v) => {
                f.write_char('[')?;
                let mut first = true;
                for value in v {
                    if !first {
                        f.write_str(", ")?;
                    }
                    first = false;
                    value.fmt(f)?;
                }
                f.write_char(']')
            }
            Value::Error(e) => e.fmt(f),
            Value::Null => f.write_str("Nil"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimpleString(arg0) => f.debug_tuple("SimpleString").field(arg0).finish(),
            Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
            Self::Double(arg0) => f.debug_tuple("Double").field(arg0).finish(),
            Self::BulkString(arg0) => f
                .debug_tuple("BulkString")
                .field(&String::from_utf8_lossy(arg0).into_owned())
                .finish(),
            Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
            Self::Array(arg0) => f.debug_tuple("Array").field(arg0).finish(),
            Self::Map(arg0) => f.debug_tuple("Map").field(arg0).finish(),
            Self::Set(arg0) => f.debug_tuple("Set").field(arg0).finish(),
            Self::Push(arg0) => f.debug_tuple("Push").field(arg0).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(arg0).finish(),
            Self::Null => write!(f, "Nil"),
        }
    }
}
