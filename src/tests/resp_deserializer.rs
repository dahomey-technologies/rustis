use crate::{
    ClientError, ErrorKind, RedisError, RedisErrorKind, Result,
    resp::{RespBuf, RespDeserializer, RespFrameParser, RespResponse, RespTapeMut, Value},
    tests::log_try_init,
};
use bytes::Bytes;
use serde::{Deserialize, Deserializer};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

fn deserialize<T>(str: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let buf = str.as_bytes();
    let mut tape = RespTapeMut::default();
    let (frame, _) = RespFrameParser::new(buf, &mut tape).parse()?;
    let response = RespResponse::new(RespBuf::from(Bytes::copy_from_slice(buf)), frame);
    deserialize_from_resp_response(response)
}

fn deserialize_from_resp_response<T>(response: RespResponse) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let deserializer = RespDeserializer::new(response.view()?);
    T::deserialize(deserializer)
}

#[test]
fn bool() -> Result<()> {
    log_try_init();

    let result: Result<bool> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: bool = deserialize("#t\r\n")?; // true
    assert!(result);

    let result: bool = deserialize("#f\r\n")?; // false
    assert!(!result);

    let result: bool = deserialize("$1\r\n1\r\n")?; // b"1"
    assert!(result);

    let result: bool = deserialize("$1\r\n0\r\n")?; // b"0"
    assert!(!result);

    let result: bool = deserialize("$4\r\ntrue\r\n")?; // b"true"
    assert!(result);

    let result: bool = deserialize("$5\r\nfalse\r\n")?; // b"false"
    assert!(!result);

    let result: bool = deserialize(":1\r\n")?; // 1
    assert!(result);

    let result: bool = deserialize(":0\r\n")?; // 0
    assert!(!result);

    let result: bool = deserialize("+OK\r\n")?; // "OK"
    assert!(result);

    let result: bool = deserialize("_\r\n")?; // nil
    assert!(!result);

    // A simple string and a bulk string carrying the same text read the same
    // way, so the RESP version a server answers in cannot flip the result.
    let result: bool = deserialize("$2\r\nOK\r\n")?;
    assert!(result);

    let result: bool = deserialize("+1\r\n")?;
    assert!(result);

    let result: bool = deserialize("+true\r\n")?;
    assert!(result);

    let result: bool = deserialize("+0\r\n")?;
    assert!(!result);

    let result: bool = deserialize("+false\r\n")?;
    assert!(!result);

    // Text the rule does not recognize is an error: the server did not say
    // `false`, and answering `false` would be inventing a value.
    for unreadable in ["+KO\r\n", "$5\r\nhello\r\n", "$0\r\n\r\n"] {
        let result: Result<bool> = deserialize(unreadable);
        let error = result.unwrap_err();
        assert!(
            matches!(
                error.kind(),
                ErrorKind::Client(ClientError::CannotParseBoolean)
            ),
            "{unreadable:?} read as a bool"
        );
    }

    Ok(())
}

#[test]
fn a_boolean_reads_the_same_off_the_wire_and_off_a_value() -> Result<()> {
    log_try_init();

    // The two deserializers share one rule, so the path the caller took cannot
    // decide whether a reply is `true`, `false` or an error.
    for reply in [
        "+OK\r\n",
        "+KO\r\n",
        "+1\r\n",
        "+false\r\n",
        "$2\r\nOK\r\n",
        "$4\r\ntrue\r\n",
        "$5\r\nhello\r\n",
        "$0\r\n\r\n",
        ":0\r\n",
        ":12\r\n",
        "#t\r\n",
        ",0\r\n",
        "_\r\n",
        "*1\r\n:1\r\n",
    ] {
        let from_wire: Result<bool> = deserialize(reply);
        let value: Value = deserialize(reply)?;
        let from_value = bool::deserialize(&value);

        match (from_wire, from_value) {
            (Ok(wire), Ok(value)) => assert_eq!(wire, value, "{reply:?}"),
            (Err(wire), Err(value)) => {
                assert_eq!(wire.to_string(), value.to_string(), "{reply:?}");
            }
            (wire, value) => {
                panic!("{reply:?} reads as {wire:?} off the wire and {value:?} off a value")
            }
        }
    }

    Ok(())
}

#[test]
fn integer() {
    log_try_init();

    let result: Result<i64> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: i64 = deserialize(":12\r\n").unwrap(); // 12
    assert_eq!(12, result);

    // A nil is not an integer: `0` is a value a key can hold.
    let result: Result<i64> = deserialize("_\r\n");
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    let result: i64 = deserialize("$2\r\n12\r\n").unwrap(); // b"12"
    assert_eq!(12, result);

    let result: i64 = deserialize("+12\r\n").unwrap(); // "12"
    assert_eq!(12, result);

    let result: i64 = deserialize("*1\r\n:12\r\n").unwrap(); // [12]
    assert_eq!(12, result);

    let result: u64 = deserialize("*1\r\n:12\r\n").unwrap(); // [12]
    assert_eq!(12, result);
}

/// A single-element array unwraps to its element, but a longer one must not
/// silently drop the extra elements.
#[test]
fn integer_from_multi_element_array() {
    log_try_init();

    macro_rules! assert_rejected {
        ($($ty:ty),+) => {$(
            let result: Result<$ty> = deserialize("*2\r\n:12\r\n:13\r\n");
            let error = result.unwrap_err();
            assert!(
                matches!(error.kind(),
                    ErrorKind::Client(ClientError::CannotParseInteger)
                ),
                "{} accepted a 2-element array",
                stringify!($ty)
            );
        )+};
    }

    assert_rejected!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);
}

#[test]
fn float() -> Result<()> {
    log_try_init();

    let result: Result<f64> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: f64 = deserialize(":12\r\n")?; // 12
    assert_eq!(12.0, result);

    let result: f64 = deserialize(",12.12\r\n")?; // 12.12
    assert_eq!(12.12, result);

    // A nil is not a number: `0.0` is a value a key can hold.
    let result: Result<f64> = deserialize("_\r\n");
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    let result: f64 = deserialize("$5\r\n12.12\r\n")?; // b"12.12"
    assert_eq!(12.12, result);

    let result: f64 = deserialize("+12.12\r\n")?; // "12.12"
    assert_eq!(12.12, result);

    Ok(())
}

#[test]
fn char() -> Result<()> {
    log_try_init();

    let result: Result<char> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: char = deserialize("$1\r\nm\r\n")?; // b"m"
    assert_eq!('m', result);

    let result: char = deserialize("+m\r\n")?; // "m"
    assert_eq!('m', result);

    Ok(())
}

#[test]
fn string() -> Result<()> {
    log_try_init();

    let result: Result<String> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: String = deserialize("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!("hello", result);

    let result: String = deserialize("+hello\r\n")?; // "hello"
    assert_eq!("hello", result);

    let result: String = deserialize("$0\r\n\r\n")?; // b""
    assert_eq!("", result);

    Ok(())
}

/// A `HASH` field, and any other `GET`-like reply, comes back as a bulk string
/// whatever it holds, so that single wire form has to reach every primitive
/// target. The cases below are the ones a hash written by another application
/// produces, and they are read strictly enough to be worth pinning: an
/// out-of-range or empty value is an error rather than a plausible number.
#[test]
fn a_bulk_string_reads_into_every_primitive() {
    log_try_init();

    fn bulk<T>(payload: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        deserialize(&format!("${}\r\n{}\r\n", payload.len(), payload))
    }

    // signed and unsigned, at every width
    assert_eq!(-8i8, bulk::<i8>("-8").unwrap());
    assert_eq!(-16i16, bulk::<i16>("-16").unwrap());
    assert_eq!(-32i32, bulk::<i32>("-32").unwrap());
    assert_eq!(-64i64, bulk::<i64>("-64").unwrap());
    assert_eq!(-128i128, bulk::<i128>("-128").unwrap());
    assert_eq!(8u8, bulk::<u8>("8").unwrap());
    assert_eq!(16u16, bulk::<u16>("16").unwrap());
    assert_eq!(32u32, bulk::<u32>("32").unwrap());
    assert_eq!(64u64, bulk::<u64>("64").unwrap());
    assert_eq!(128u128, bulk::<u128>("128").unwrap());
    assert_eq!(u64::MAX, bulk::<u64>("18446744073709551615").unwrap());

    // floats, including the notations Redis itself emits for a score
    assert_eq!(1.75f32, bulk::<f32>("1.75").unwrap());
    assert_eq!(1.75f64, bulk::<f64>("1.75").unwrap());
    assert_eq!(1750f64, bulk::<f64>("1.75e3").unwrap());
    assert_eq!(f64::INFINITY, bulk::<f64>("inf").unwrap());
    assert!(bulk::<f64>("nan").unwrap().is_nan());

    // the booleans rustis writes, plus the two spellings a human would
    assert!(bulk::<bool>("1").unwrap());
    assert!(!bulk::<bool>("0").unwrap());
    assert!(bulk::<bool>("true").unwrap());
    assert!(!bulk::<bool>("false").unwrap());

    assert_eq!('m', bulk::<char>("m").unwrap());
    assert_eq!("hello", bulk::<String>("hello").unwrap());

    // a nil is the only `None`: an empty bulk string is a value, and an empty one
    assert_eq!(Some(String::new()), bulk::<Option<String>>("").unwrap());
    assert_eq!(None, deserialize::<Option<String>>("_\r\n").unwrap());

    macro_rules! assert_rejected {
        ($ty:ty, $payload:expr, $err:pat) => {
            let error = bulk::<$ty>($payload).unwrap_err();
            assert!(
                matches!(error.kind(), ErrorKind::Client($err)),
                "{} accepted {:?}: {error:?}",
                stringify!($ty),
                $payload
            );
        };
    }

    // out of the target's range, rather than wrapped or saturated
    assert_rejected!(u8, "300", ClientError::CannotParseInteger);
    assert_rejected!(i64, "9223372036854775808", ClientError::CannotParseInteger);
    assert_rejected!(u32, "-1", ClientError::CannotParseInteger);
    // nothing to parse: an empty field is not a zero
    assert_rejected!(u32, "", ClientError::CannotParseInteger);
    assert_rejected!(f64, "", ClientError::CannotParseDouble);
    assert_rejected!(bool, "", ClientError::CannotParseBoolean);
    assert_rejected!(char, "", ClientError::CannotParseChar);
    // not that value at all
    assert_rejected!(f64, "1.75abc", ClientError::CannotParseDouble);
    assert_rejected!(bool, "2", ClientError::CannotParseBoolean);
    assert_rejected!(bool, "TRUE", ClientError::CannotParseBoolean);
    assert_rejected!(char, "ab", ClientError::CannotParseChar);
}

/// A text reply that is not entirely an integer is rejected, rather than read as
/// however much of it parses. `atoi` alone stops at the first byte that is not a
/// digit, which turns `1.75` into `1` and `0x10` into `0`: a value the server
/// never sent, indistinguishable from one it did.
#[test]
fn a_text_reply_read_as_an_integer_is_all_or_nothing() {
    log_try_init();

    macro_rules! assert_rejected {
        ($ty:ty, $reply:expr) => {
            let error = deserialize::<$ty>($reply).unwrap_err();
            assert!(
                matches!(
                    error.kind(),
                    ErrorKind::Client(ClientError::CannotParseInteger)
                ),
                "{} accepted {:?}: {error:?}",
                stringify!($ty),
                $reply
            );
        };
    }

    // a trailing remainder, whatever it is
    assert_rejected!(u32, "$5\r\n12abc\r\n");
    assert_rejected!(u32, "$4\r\n1.75\r\n");
    assert_rejected!(i32, "$4\r\n0x10\r\n");
    assert_rejected!(i32, "$3\r\n1 2\r\n");
    // a leading one, which `atoi` reports as nothing read
    assert_rejected!(u32, "$3\r\n 12\r\n");
    // a sign and no digits, which it reports as one byte read for a 0
    assert_rejected!(i32, "$1\r\n-\r\n");
    assert_rejected!(i32, "$1\r\n+\r\n");
    // the same rule on a simple string, and on the digits behind a `:`
    assert_rejected!(u32, "+12abc\r\n");
    assert_rejected!(u32, ":12abc\r\n");

    // what stays accepted: a plain integer, either sign, at any width
    assert_eq!(12u32, deserialize::<u32>("$2\r\n12\r\n").unwrap());
    assert_eq!(-12i32, deserialize::<i32>("$3\r\n-12\r\n").unwrap());
    // RESP3 allows an explicit `+` on an integer reply
    assert_eq!(12u32, deserialize::<u32>("$3\r\n+12\r\n").unwrap());
    assert_eq!(
        i64::MIN,
        deserialize::<i64>("$20\r\n-9223372036854775808\r\n").unwrap()
    );
}

/// A numeric reply read as a `String` gives back the bytes the server sent,
/// verbatim. Decoding the number and re-rendering it would not round-trip: the
/// text Redis chose carries precision and notation that an `f64` does not keep.
#[test]
fn a_numeric_reply_read_as_a_string_is_verbatim() -> Result<()> {
    log_try_init();

    let result: String = deserialize(":12\r\n")?;
    assert_eq!("12", result);

    let result: String = deserialize(":-9223372036854775808\r\n")?;
    assert_eq!("-9223372036854775808", result);

    // A trailing zero is significant to whoever sent it, and re-rendering the
    // `f64` would drop it.
    let result: String = deserialize(",12.50\r\n")?;
    assert_eq!("12.50", result);

    // Redis's own notation is preserved in both directions: an exponent is not
    // expanded, and an integral double keeps no spurious `.0`.
    let result: String = deserialize(",1e21\r\n")?;
    assert_eq!("1e21", result);

    let result: String = deserialize(",12\r\n")?;
    assert_eq!("12", result);

    let result: String = deserialize(",inf\r\n")?;
    assert_eq!("inf", result);

    // A big number is already surfaced as its digits, and stays exact.
    let result: String = deserialize("(3492890328409238509324850943850943825024385\r\n")?;
    assert_eq!("3492890328409238509324850943850943825024385", result);

    Ok(())
}

/// A reply the client synthesized never came off the wire, so it has no bytes to
/// hand back and renders from its value instead.
#[test]
fn a_synthesized_numeric_reply_renders_from_its_value() -> Result<()> {
    log_try_init();

    let result: String = deserialize_from_resp_response(RespResponse::integer(42))?;
    assert_eq!("42", result);

    Ok(())
}

/// Captures whatever text a deserialize method hands a visitor, so the two string
/// entry points can be compared directly instead of through a guess about which
/// target type serde routes where.
struct CaptureStr;

impl<'de> serde::de::Visitor<'de> for CaptureStr {
    type Value = String;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a string")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<String, E> {
        Ok(v.to_owned())
    }
}

/// `deserialize_str` and `deserialize_string` answer the same question, so they
/// must answer it identically. They are reached by different targets — the former
/// through `deserialize_identifier`, which serde uses for struct field names and
/// enum variant names, the latter for a `String` — so a disagreement means the
/// type a caller picked decides whether their command succeeds.
#[test]
fn the_two_string_entry_points_agree() -> Result<()> {
    log_try_init();

    let cases = [
        "+hello\r\n",
        "$5\r\nhello\r\n",
        "$0\r\n\r\n",
        ":12\r\n",
        ",12.50\r\n",
        "#t\r\n",
        "#f\r\n",
        "_\r\n",
    ];

    for resp in cases {
        let buf = resp.as_bytes();
        let mut tape = RespTapeMut::default();
        let (frame, _) = RespFrameParser::new(buf, &mut tape).parse()?;
        let response = RespResponse::new(RespBuf::from(Bytes::copy_from_slice(buf)), frame);

        let as_str =
            Deserializer::deserialize_str(RespDeserializer::new(response.view()?), CaptureStr);
        let as_string =
            Deserializer::deserialize_string(RespDeserializer::new(response.view()?), CaptureStr);

        match (as_str, as_string) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "different text for {resp:?}"),
            (Err(_), Err(_)) => {}
            (a, b) => panic!("{resp:?} readable through one entry point only: {a:?} / {b:?}"),
        }
    }

    // Same for a reply the client synthesized, which has no bytes to borrow.
    let response = RespResponse::integer(42);
    let as_str =
        Deserializer::deserialize_str(RespDeserializer::new(response.view()?), CaptureStr)?;
    let as_string =
        Deserializer::deserialize_string(RespDeserializer::new(response.view()?), CaptureStr)?;
    assert_eq!(as_str, as_string);
    assert_eq!("42", as_str);

    Ok(())
}

/// Non-numeric scalars keep their rendering: `#t` is a boolean, and `"t"` would
/// be a worse answer than `"true"` for the caller that asked for text.
#[test]
fn a_boolean_reply_read_as_a_string_renders_as_true_or_false() -> Result<()> {
    log_try_init();

    let result: String = deserialize("#t\r\n")?;
    assert_eq!("true", result);

    let result: String = deserialize("#f\r\n")?;
    assert_eq!("false", result);

    // A nil is not a string: `""` is a value a key can hold.
    let result: Result<String> = deserialize("_\r\n");
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));

    Ok(())
}

#[test]
fn option() -> Result<()> {
    log_try_init();

    let result: Result<Option<String>> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Option<String> = deserialize("$5\r\nhello\r\n")?; // b"hello"
    assert_eq!(Some("hello".to_owned()), result);

    let result: Option<String> = deserialize("$0\r\n\r\n")?; // b""
    assert_eq!(Some("".to_owned()), result);

    let result: Option<String> = deserialize("_\r\n")?; // null
    assert_eq!(None, result);

    let result: Option<i64> = deserialize(":12\r\n")?; // b"12"
    assert_eq!(Some(12), result);

    let result: Option<i64> = deserialize("_\r\n")?; // null
    assert_eq!(None, result);

    let result: Option<Vec<i32>> = deserialize("*1\r\n:12\r\n")?; // [12]
    assert_eq!(Some(vec![12]), result);

    // An empty collection is a collection, not a nil: only `_` and `*-1` are `None`.
    let result: Option<Vec<i32>> = deserialize("*0\r\n")?; // []
    assert_eq!(Some(vec![]), result);

    let result: Option<Vec<i32>> = deserialize("*-1\r\n")?; // nil array
    assert_eq!(None, result);

    let result: Option<Vec<i32>> = deserialize("_\r\n")?; // null
    assert_eq!(None, result);

    let result: Option<HashMap<String, i32>> = deserialize("%0\r\n")?; // {}
    assert_eq!(Some(HashMap::new()), result);

    let result: Option<HashSet<i32>> = deserialize("~0\r\n")?; // set()
    assert_eq!(Some(HashSet::new()), result);

    Ok(())
}

#[test]
fn unit() -> Result<()> {
    log_try_init();

    let result: Result<()> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Result<()> = deserialize("+OK\r\n"); // "OK"
    assert!(result.is_ok());

    let result: Result<()> = deserialize("_\r\n"); // null
    assert!(result.is_ok());

    let result: Result<()> = deserialize("$5\r\nhello\r\n"); // "hello"
    assert!(result.is_ok());

    let result: Result<()> = deserialize(":1\r\n"); // 1
    assert!(result.is_ok(), "{result:?}");

    Ok(())
}

#[test]
fn unit_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize, Debug)]
    struct Unit;

    let result: Result<Unit> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Result<Unit> = deserialize("_\r\n"); // null
    assert!(result.is_ok());

    let result: Result<Unit> = deserialize("$5\r\nhello\r\n"); // "hello"
    assert!(result.is_ok());

    Ok(())
}

#[test]
fn newtype_struct() -> Result<()> {
    log_try_init();

    #[derive(Deserialize, Debug)]
    struct Millimeters(u8);

    let result: Result<Millimeters> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Millimeters = deserialize(":12\r\n")?; // 12
    assert_eq!(12, result.0);

    Ok(())
}

#[test]
fn seq() -> Result<()> {
    log_try_init();

    let result: Result<Vec<i32>> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Vec<i32> = deserialize("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!(2, result.len());
    assert_eq!(12, result[0]);
    assert_eq!(13, result[1]);

    let result: Vec<i32> = deserialize("*2\r\n$2\r\n12\r\n$2\r\n13\r\n")?; // [b "12", b"13"]
    assert_eq!(2, result.len());
    assert_eq!(12, result[0]);
    assert_eq!(13, result[1]);

    let result: Vec<bool> = deserialize("*2\r\n#t\r\n#f\r\n")?; // [true, false]
    assert_eq!(2, result.len());
    assert!(result[0]);
    assert!(!result[1]);

    let result: Vec<bool> = deserialize("*2\r\n:1\r\n:0\r\n")?; // [1, 0]
    assert_eq!(2, result.len());
    assert!(result[0]);
    assert!(!result[1]);

    let result: SmallVec<[String; 2]> = deserialize("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello", b"world"]
    assert_eq!(2, result.len());
    assert_eq!("hello", result[0]);
    assert_eq!("world", result[1]);

    Ok(())
}

#[test]
fn integer_array() {
    log_try_init();

    let result: Vec<i32> =
        deserialize_from_resp_response(RespResponse::IntegerArray(vec![12, 13])).unwrap();
    assert_eq!(2, result.len());
    assert_eq!(12, result[0]);
    assert_eq!(13, result[1]);

    let result: Vec<bool> =
        deserialize_from_resp_response(RespResponse::IntegerArray(vec![1, 0])).unwrap();
    assert_eq!(2, result.len());
    assert!(result[0]);
    assert!(!result[1]);
}

#[test]
fn tuple() -> Result<()> {
    log_try_init();

    let result: Result<(i32, i32)> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Result<(i32, i32)> = deserialize("!9\r\nERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: (i32, i32) = deserialize("*2\r\n:12\r\n:13\r\n")?; // [12, 13]
    assert_eq!((12, 13), result);

    let result: (String, String) = deserialize("*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n")?; // [b"hello", b"world"]
    assert_eq!(("hello".to_string(), "world".to_string()), result);

    Ok(())
}

#[test]
fn tuple_struct() -> Result<()> {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Rgb(u8, u8, u8);

    let result: Result<Rgb> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Rgb = deserialize("*3\r\n:12\r\n:13\r\n:14\r\n")?; // [12, 13, 14]
    assert_eq!(Rgb(12, 13, 14), result);

    Ok(())
}

#[test]
fn map() {
    log_try_init();

    let result: Result<HashMap<i32, i32>> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: HashMap<i32, i32> = deserialize("*4\r\n:12\r\n:13\r\n:14\r\n:15\r\n").unwrap(); // [12, 13, 14, 15]
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result: HashMap<i32, i32> = deserialize("%2\r\n:12\r\n:13\r\n:14\r\n:15\r\n").unwrap(); // { 12: 13, 14: 15 }
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));

    let result: HashMap<i32, i32> =
        deserialize("*2\r\n*2\r\n:12\r\n:13\r\n*2\r\n:14\r\n:15\r\n").unwrap(); // [[12, 13], [14, 15]]
    assert_eq!(Some(&13), result.get(&12));
    assert_eq!(Some(&15), result.get(&14));
}

#[test]
fn r#struct() {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Person {
        pub id: u64,
        pub name: String,
    }

    let result: Result<Person> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result: Person =
        deserialize("*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n").unwrap(); // [b"id", 12, b"name", b"Mike"]
    assert_eq!(
        Person {
            id: 12,
            name: "Mike".to_owned()
        },
        result
    );

    let result: Person =
        deserialize("%2\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n").unwrap(); // {b"id": 12, b"name": b"Mike"}
    assert_eq!(
        Person {
            id: 12,
            name: "Mike".to_owned()
        },
        result
    );

    let result: Person = deserialize("*2\r\n:12\r\n$4\r\nMike\r\n").unwrap(); // [12, b"Mike"]
    assert_eq!(
        Person {
            id: 12,
            name: "Mike".to_owned()
        },
        result
    );
}

/// The shapes a flat array can take must decode the same way whatever the
/// server adds, removes, or renders as a simple string. Mirrored, case for
/// case, by `value_deserializer::struct_flat_array_shapes`.
#[test]
fn struct_flat_array_shapes() {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Person {
        id: u64,
        name: String,
    }

    /// Two required fields and two optional ones, to exercise a server that
    /// answers fewer field/value pairs than the struct declares.
    #[derive(Debug, Deserialize, PartialEq)]
    struct PartialPerson {
        id: u64,
        name: String,
        ttl: Option<u64>,
        tag: Option<String>,
    }

    let mike = Person {
        id: 12,
        name: "Mike".to_owned(),
    };

    // A field the server added to a field/value array is ignored.
    let result: Person =
        deserialize("*6\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n$3\r\nttl\r\n:99\r\n")
            .unwrap();
    assert_eq!(mike, result);

    // Field names rendered as simple strings are field names too.
    let result: Person = deserialize("*4\r\n+id\r\n:12\r\n+name\r\n$4\r\nMike\r\n").unwrap();
    assert_eq!(mike, result);

    // An element the server appended to a positional array is ignored.
    let result: Person = deserialize("*3\r\n:12\r\n$4\r\nMike\r\n:99\r\n").unwrap();
    assert_eq!(mike, result);

    // Two pairs for a four-field struct are pairs, not a positional tuple:
    // the absent fields are left to their `Option` default.
    let result: PartialPerson =
        deserialize("*4\r\n$2\r\nid\r\n:12\r\n$4\r\nname\r\n$4\r\nMike\r\n").unwrap();
    assert_eq!(
        PartialPerson {
            id: 12,
            name: "Mike".to_owned(),
            ttl: None,
            tag: None
        },
        result
    );

    // A missing required field is a serde error naming it, not a blanket
    // `CannotParseStruct`.
    let result: Result<Person> = deserialize("*2\r\n$2\r\nid\r\n:12\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(error.kind(), ErrorKind::Client(ClientError::SerdeDeserialize(msg)) if msg.contains("name")),
        "{error:?}"
    );
}

/// Structs also decode from the two synthesized array shapes — the cluster
/// aggregates fan-out replies into an owned array, the client-side cache
/// rebuilds `MGET` the same way.
#[test]
fn struct_from_synthesized_arrays() {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    struct Pair {
        first: i64,
        second: i64,
    }

    let result: Pair = deserialize_from_resp_response(RespResponse::integer_array(vec![12, 13]))
        .expect("integer array");
    assert_eq!(
        Pair {
            first: 12,
            second: 13
        },
        result
    );

    let result: Pair = deserialize_from_resp_response(RespResponse::owned_array(vec![
        RespResponse::integer(12),
        RespResponse::integer(13),
    ]))
    .expect("owned array");
    assert_eq!(
        Pair {
            first: 12,
            second: 13
        },
        result
    );
}

#[test]
fn r#enum() {
    log_try_init();

    #[derive(Debug, Deserialize, PartialEq)]
    enum E {
        A,                         // unit_variant
        B(u8),                     // newtype_variant
        C(u8, u8),                 // tuple_variant
        D { r: u8, g: u8, b: u8 }, // struct_variant
    }

    let result: Result<E> = deserialize("-ERR error\r\n"); // error
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    // unit_variant
    let result: E = deserialize("$1\r\nA\r\n").unwrap(); // b"A"
    assert_eq!(E::A, result);

    let result: E = deserialize("+A\r\n").unwrap(); // "A"
    assert_eq!(E::A, result);

    // newtype_variant
    let result: E = deserialize("*2\r\n$1\r\nB\r\n:12\r\n").unwrap(); // [ b"B", 12 ]
    assert_eq!(E::B(12), result);

    let result: E = deserialize("%1\r\n$1\r\nB\r\n:12\r\n").unwrap(); // { b"B": 12 }
    assert_eq!(E::B(12), result);

    // tuple_variant
    let result: E = deserialize("*2\r\n$1\r\nC\r\n*2\r\n:12\r\n:13\r\n").unwrap(); // [ b"C", [12, 13] ]
    assert_eq!(E::C(12, 13), result);

    let result: E = deserialize("%1\r\n$1\r\nC\r\n*2\r\n:12\r\n:13\r\n").unwrap(); // { b"C": [12, 13] }
    assert_eq!(E::C(12, 13), result);

    // struct_variant
    let result: E = deserialize(
        "*2\r\n$1\r\nD\r\n*6\r\n$1\r\nr\r\n:12\r\n$1\r\ng\r\n:13\r\n$1\r\nb\r\n:14\r\n",
    )
    .unwrap(); // [ b"D", [b"r", 12, b"g", 13, b"b", 14] ]
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    let result: E = deserialize(
        "%1\r\n$1\r\nD\r\n*6\r\n$1\r\nr\r\n:12\r\n$1\r\ng\r\n:13\r\n$1\r\nb\r\n:14\r\n",
    )
    .unwrap(); // { b"D", [b"r", 12, b"g", 13, b"b", 14] }
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );

    let result: E = deserialize(
        "%1\r\n$1\r\nD\r\n%3\r\n$1\r\nr\r\n:12\r\n$1\r\ng\r\n:13\r\n$1\r\nb\r\n:14\r\n",
    )
    .unwrap(); // { b"D", { b"r": 12, b"g": 13, b"b": 14 } }
    assert_eq!(
        E::D {
            r: 12,
            g: 13,
            b: 14
        },
        result
    );
}

/// A map whose value re-parses as a malformed scalar (framing does not validate
/// integer/double content) must surface a decode error, not panic the caller's
/// task on the pair-iteration `unwrap`s.
#[test]
fn map_malformed_nested_scalar_errors() -> Result<()> {
    log_try_init();

    // `*1\r\n*2\r\n:abc\r\n:1\r\n`: a map read from a sequence of 2-element pairs
    // where the key element's bytes `abc` pass framing but fail `atoi` when the
    // pair iterator re-reads the element (`read_scalar_view` → None).
    let result: Result<HashMap<i64, i64>> = deserialize("*1\r\n*2\r\n:abc\r\n:1\r\n");
    assert!(result.is_err(), "expected a decode error, got {result:?}");

    Ok(())
}

/// `Display`/`Debug` of a `RespBuf` is used by trace/debug logging and must never
/// panic — not on an empty buffer (produced by `null()`/`integer()` values), and
/// not allocate the whole reply for a large buffer (it summarizes past a limit).
#[test]
fn resp_buf_display_never_panics() {
    log_try_init();

    // Empty buffer: the parser returns EOF instead of indexing out of bounds.
    let empty = RespBuf::from(Bytes::new());
    let _ = format!("{empty}");
    let _ = format!("{empty:?}");

    // Large buffer: summarized rather than fully materialized.
    let big = RespBuf::from(Bytes::from(vec![b'x'; 64 * 1024]));
    let rendered = format!("{big}");
    assert!(
        rendered.contains("RESP buffer of"),
        "large buffer should be summarized, got {rendered}"
    );
}

/// An empty RESP array/map must decode to an empty `Value` collection, distinct
/// from a nil (`*-1` / `_`) which stays `Value::Null`. Collapsing both to `Null`
/// destroys the empty-vs-nil distinction that `Value` exists to preserve.
#[test]
fn empty_collections_are_not_null() {
    use crate::resp::Value;
    log_try_init();

    assert_eq!(
        Value::Array(vec![]),
        deserialize::<Value>("*0\r\n").unwrap()
    );
    assert!(matches!(
        deserialize::<Value>("%0\r\n").unwrap(),
        Value::Map(m) if m.is_empty()
    ));

    // Nil must still be Null, not an empty collection.
    assert_eq!(Value::Null, deserialize::<Value>("_\r\n").unwrap());
    assert_eq!(Value::Null, deserialize::<Value>("*-1\r\n").unwrap());
}

/// A RESP3 map with a non-string key (boolean, array, …) is protocol-valid, so
/// `Value` must carry such a key as it stands and let it be looked up.
#[test]
fn value_map_with_boolean_key_is_readable() -> Result<()> {
    use crate::resp::Value;
    log_try_init();

    // `%1\r\n#t\r\n:1\r\n`: { true: 1 }.
    let value: Value = deserialize("%1\r\n#t\r\n:1\r\n")?;
    assert_eq!(Some(&Value::Integer(1)), value.get(&Value::Boolean(true)));

    Ok(())
}

#[test]
fn out_of_range_integer_errors_instead_of_truncating() {
    log_try_init();

    // `:300` no longer silently truncates to 44 when deserialized as u8.
    let result: Result<u8> = deserialize(":300\r\n");
    assert!(
        matches!(
            result.unwrap_err().kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "u8 from 300 should error"
    );

    // A negative wire integer into an unsigned target no longer wraps.
    let result: Result<u32> = deserialize(":-1\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "u32 from -1 should error, got {error:?}"
    );

    // In-range values still deserialize.
    let value: u8 = deserialize(":42\r\n").unwrap();
    assert_eq!(42, value);
}

#[test]
fn i64_min_is_parsed_not_rejected() {
    log_try_init();

    // DECRBY can return i64::MIN; the frame parser accumulates negatively so the
    // value is representable rather than rejected as an overflow.
    let value: i64 = deserialize(":-9223372036854775808\r\n").unwrap();
    assert_eq!(i64::MIN, value);

    let value: i64 = deserialize(":9223372036854775807\r\n").unwrap();
    assert_eq!(i64::MAX, value);
}

#[test]
fn null_does_not_deserialize_to_an_integer_default() {
    log_try_init();

    // A nil reply carries the absence to the caller. `Option<i32>` accepts it;
    // `i32` cannot, `0` being a value a key can hold.
    let result: Result<i32> = deserialize("_\r\n");
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Client(ClientError::UnexpectedNil { .. })
    ));
    assert_eq!(None, deserialize::<Option<i32>>("_\r\n").unwrap());
}

#[test]
fn lossy_double_to_integer_errors_instead_of_truncating() {
    log_try_init();

    // A fractional score has no exact integer value.
    let result: Result<i64> = deserialize(",3.9\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "i64 from 3.9 should error, got {error:?}"
    );

    // Beyond the target's range, where an `as` cast would saturate.
    let result: Result<i64> = deserialize(",1e300\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "i64 from 1e300 should error, got {error:?}"
    );

    // Out of range for the narrow target, in range for f64.
    let result: Result<i8> = deserialize(",300\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "i8 from 300 should error, got {error:?}"
    );

    // A negative double has no value in an unsigned target.
    let result: Result<u32> = deserialize(",-1\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "u32 from -1.0 should error, got {error:?}"
    );

    // NaN and the infinities are not integers.
    let result: Result<i64> = deserialize(",nan\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "i64 from NaN should error, got {error:?}"
    );
    let result: Result<i64> = deserialize(",inf\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "i64 from inf should error, got {error:?}"
    );

    // An exactly representable double still deserializes to the integer.
    let value: i64 = deserialize(",3\r\n").unwrap();
    assert_eq!(3, value);
    let value: u8 = deserialize(",255.0\r\n").unwrap();
    assert_eq!(255, value);
    let value: i128 = deserialize(",-42\r\n").unwrap();
    assert_eq!(-42, value);
}

#[test]
fn negative_integer_to_u128_errors_instead_of_wrapping() {
    log_try_init();

    // A negative wire integer has no `u128` value.
    let result: Result<u128> = deserialize(":-1\r\n");
    let error = result.unwrap_err();
    assert!(
        matches!(
            error.kind(),
            ErrorKind::Client(ClientError::CannotParseInteger)
        ),
        "u128 from -1 should error, got {error:?}"
    );

    let value: u128 = deserialize(":42\r\n").unwrap();
    assert_eq!(42, value);
}

/// A RESP3 map is an ordered sequence of field/value pairs on the wire, and the
/// server is free to send the same field twice. `Value` is the untyped fallback:
/// it must hand back what arrived, in the order it arrived, so a caller reading
/// a reply whose shape it does not model sees the reply itself.
#[test]
fn a_map_keeps_the_reply_order_and_its_duplicate_keys() -> Result<()> {
    log_try_init();

    // `{ b: 1, a: 2, a: 3 }` — out of sorted order, and `a` twice.
    let value: Value = deserialize("%3\r\n+b\r\n:1\r\n+a\r\n:2\r\n+a\r\n:3\r\n")?;
    let Value::Map(entries) = value else {
        panic!("expected a Value::Map");
    };

    assert_eq!(
        vec![
            (Value::SimpleString("b".to_owned()), Value::Integer(1)),
            (Value::SimpleString("a".to_owned()), Value::Integer(2)),
            (Value::SimpleString("a".to_owned()), Value::Integer(3)),
        ],
        entries
    );

    Ok(())
}

/// A nil read as a scalar is refused, not coerced.
///
/// Redis answers nil for a key that does not exist. Read as a number it used to
/// be `0`, as a string `""`, as a char `'\0'` — each indistinguishable from the
/// value a present key holds, and each returned forever without a signal.
#[test]
fn a_nil_read_as_a_scalar_is_an_error() -> Result<()> {
    log_try_init();

    fn refused<T: serde::de::DeserializeOwned + std::fmt::Debug>(target: &str) {
        let result: Result<T> = deserialize("_\r\n");
        let error = result.expect_err("a nil is not a {target}");
        assert!(
            matches!(
                error.kind(),
                ErrorKind::Client(ClientError::UnexpectedNil { target: named }) if *named == target
            ),
            "reading a nil as {target} gave {error:?}"
        );
    }

    refused::<i8>("an integer");
    refused::<i16>("an integer");
    refused::<i32>("an integer");
    refused::<i64>("an integer");
    refused::<i128>("an integer");
    refused::<u8>("an integer");
    refused::<u16>("an integer");
    refused::<u32>("an integer");
    refused::<u64>("an integer");
    refused::<u128>("an integer");
    refused::<f32>("a floating-point number");
    refused::<f64>("a floating-point number");
    refused::<char>("a char");
    refused::<String>("a string");

    Ok(())
}

/// `Option<R>` is the type that accepts the absence, and the escape hatch the
/// error message names. `Value` keeps carrying it as `Value::Nil`.
#[test]
fn a_nil_still_reads_as_an_option_or_a_value() -> Result<()> {
    log_try_init();

    assert_eq!(None, deserialize::<Option<i64>>("_\r\n")?);
    assert_eq!(None, deserialize::<Option<f64>>("_\r\n")?);
    assert_eq!(None, deserialize::<Option<String>>("_\r\n")?);
    assert_eq!(None, deserialize::<Option<char>>("_\r\n")?);
    assert_eq!(Value::Null, deserialize::<Value>("_\r\n")?);

    Ok(())
}

/// The two deserializers must refuse it alike, the rule being one rule.
#[test]
fn a_nil_reads_the_same_off_the_wire_and_off_a_value() -> Result<()> {
    log_try_init();

    let value: Value = deserialize("_\r\n")?;

    let from_wire = deserialize::<i64>("_\r\n").unwrap_err();
    let from_value = i64::deserialize(&value).unwrap_err();
    assert_eq!(from_wire.to_string(), from_value.to_string());

    let from_wire = deserialize::<String>("_\r\n").unwrap_err();
    let from_value = String::deserialize(&value).unwrap_err();
    assert_eq!(from_wire.to_string(), from_value.to_string());

    assert_eq!(None, Option::<i64>::deserialize(&value)?);

    Ok(())
}
