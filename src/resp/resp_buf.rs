use crate::{
    Result,
    resp::{
        BULK_ERROR_TAG, RespDeserializer, RespFrameParser, RespResponse, RespTapeMut,
        SIMPLE_ERROR_TAG, Value,
    },
};
use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::{fmt, ops::Deref};

/// Represents a [RESP](https://redis.io/docs/reference/protocol-spec/) Buffer incoming from the network
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct RespBuf(Bytes);

impl RespBuf {
    /// Constructs a new `RespBuf` from a byte slice
    #[inline]
    pub fn from_slice(data: &[u8]) -> RespBuf {
        RespBuf(Bytes::copy_from_slice(data))
    }

    /// Returns `true` if the RESP Buffer is a Redis error
    #[inline]
    pub fn is_error(&self) -> bool {
        matches!(self.0.as_ref(), [SIMPLE_ERROR_TAG | BULK_ERROR_TAG, _, ..])
    }

    /// Convert the RESP Buffer to a Rust type `T` by using serde deserialization
    #[inline]
    pub fn to<T: DeserializeOwned>(&self) -> Result<T> {
        let mut tape = RespTapeMut::default();
        let (frame, len) = RespFrameParser::new(&self.0, &mut tape).parse()?;
        // Slice to the frame the parser actually read — a refcount bump, not a
        // copy. `RespResponse` requires a tapeless frame to end where its scalar
        // does, which a buffer holding trailing bytes would break.
        let response = RespResponse::new(RespBuf(self.0.slice(..len)), frame);
        T::deserialize(RespDeserializer::new(response.view()?))
    }

    #[inline(always)]
    pub fn bytes(&self) -> &Bytes {
        &self.0
    }

    /// Transform into Bytes
    #[inline(always)]
    pub fn into_bytes(self) -> Bytes {
        self.0
    }
}

impl Deref for RespBuf {
    type Target = Bytes;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Above this raw buffer size, `Display`/`Debug` no longer deserializes the whole
/// reply into a `Value` just to truncate the rendering to 1000 chars — a large
/// reply under debug/trace logging would otherwise trigger an allocation storm.
/// Beyond the threshold we print a cheap summary instead.
const DISPLAY_MATERIALIZE_LIMIT: usize = 4 * 1024;

impl fmt::Display for RespBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deserializing to `Value` and formatting it allocates proportionally to
        // the reply; cap that work by summarizing anything past the limit rather
        // than materializing it and truncating after the fact.
        if self.0.len() > DISPLAY_MATERIALIZE_LIMIT {
            return f.write_fmt(format_args!("<RESP buffer of {} bytes>", self.0.len()));
        }
        match self.to::<Value>() {
            Ok(value) => {
                let str = format!("{value:?}");
                if str.len() > 1000 {
                    f.write_str(str.get(..1000).unwrap_or("<can't slice to display>"))
                } else {
                    f.write_str(&str)
                }
            }
            Err(e) => f.write_fmt(format_args!("RESP buffer error: {e:?}")),
        }
    }
}

impl fmt::Debug for RespBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self, f)
    }
}

impl From<Bytes> for RespBuf {
    #[inline(always)]
    fn from(value: Bytes) -> Self {
        RespBuf(value)
    }
}
