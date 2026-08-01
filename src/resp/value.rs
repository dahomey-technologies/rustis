use crate::{RedisError, Result};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter, Write},
    hash::{Hash, Hasher},
};

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
    /// Equality on this variant is total, as `Value` is `Eq` and usable as a
    /// [`Value::Map`] key: all NaNs are equal to each other — so a `,nan` reply
    /// equals itself — and `-0.0` equals `0.0`. Both depart from IEEE-754,
    /// which has no reflexive NaN.
    Double(f64),
    /// [RESP Bulk String](https://redis.io/docs/latest/develop/reference/protocol-spec/#bulk-strings)
    BulkString(Vec<u8>),
    /// [RESP Boolean](https://redis.io/docs/latest/develop/reference/protocol-spec/#booleans)
    Boolean(bool),
    /// [RESP Array](https://redis.io/docs/latest/develop/reference/protocol-spec/#arrays)
    Array(Vec<Value>),
    /// [RESP Map](https://redis.io/docs/latest/develop/reference/protocol-spec/#maps)
    Map(HashMap<Value, Value>),
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
    /// Any parsing error ([`Error::Client`](crate::Error::Client)) due to incompatibility between Value variant and taget type
    #[inline]
    pub fn into<T>(self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(&self)
    }
}

/// Canonical bit pattern a [`Value::Double`] is compared and hashed on.
///
/// `Value` asserts `Eq`, which demands a reflexive equality, so the IEEE-754
/// rules are not usable as they stand: every NaN collapses onto a single
/// pattern, and the two zeros — equal under `==` — onto the positive one.
/// `Hash` and `PartialEq` share this function so they cannot drift apart.
fn canonical_double_bits(d: f64) -> u64 {
    if d.is_nan() {
        f64::NAN.to_bits()
    } else if d == 0.0 {
        0.0f64.to_bits()
    } else {
        d.to_bits()
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // A RESP3 map is `HashMap<Value, Value>`, so any variant can appear as a
        // key and gets hashed — including Boolean/Array/Map/Set/Push reached from
        // server data. Missing arms used to `unimplemented!()` and panic the
        // decoding task, so every variant must hash. Mixing the discriminant in
        // keeps values of different variants from colliding.
        core::mem::discriminant(self).hash(state);
        match self {
            Value::SimpleString(s) => s.hash(state),
            Value::Integer(i) => i.hash(state),
            Value::Double(d) => canonical_double_bits(*d).hash(state),
            Value::BulkString(bs) => bs.hash(state),
            Value::Boolean(b) => b.hash(state),
            Value::Array(v) | Value::Set(v) | Value::Push(v) => v.hash(state),
            Value::Map(m) => {
                // `HashMap` has no `Hash`; fold order-independently so equal maps
                // hash equally regardless of iteration order.
                let mut acc: u64 = 0;
                for (k, val) in m {
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    k.hash(&mut h);
                    val.hash(&mut h);
                    acc = acc.wrapping_add(h.finish());
                }
                acc.hash(state);
            }
            Value::Error(e) => e.hash(state),
            Value::Null => "_\r\n".hash(state),
        }
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
            Value::Map(m) => {
                f.write_char('{')?;
                let mut first = true;
                for (key, value) in m {
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
