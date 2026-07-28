use serde::{
    Deserialize,
    de::{self},
    ser::{self, Serialize},
};

/// Wrapper type that converts a Rust value from and to a Redis bulk string holding JSON.
///
/// This is useful for storing and retrieving structured data as JSON.
/// Typically used with commands like `GET` / `SET`, `HGET` / `HSET`, or any
/// command taking or returning a bulk string.
///
/// A key that may be missing must be read as `Option<Json<T>>`: a nil reply is
/// not a JSON document.
///
/// `Json(&value)` borrows and `Json(value)` moves — both serialize identically,
/// since `&T` is itself `Serialize`.
///
/// # Example
/// ```rust
/// use rustis::{
///     client::Client,
///     commands::{FlushingMode, ServerCommands, StringCommands},
///     resp::Json,
///     Result
/// };
///
/// #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = Client::connect("127.0.0.1:6379").await?;
///     client.flushall(FlushingMode::Sync).await?;
///     let user1 = User { id: 12, name: "foo".to_string() };
///     client.set("user:123", Json(&user1)).await?;
///     let Json(user2): Json<User> = client.get("user:123").await?;
///
///     assert_eq!(user1, user2);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Returns the wrapped value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

const TRANSIENT_INPUT: &str = "`Json<T>` needs data borrowed from the connection buffer, and this \
                               deserializer supplied owned data; use `serde_json` directly";

impl<'de, T> Deserialize<'de> for Json<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::{fmt, marker::PhantomData};

        struct Visitor<T> {
            phantom: PhantomData<T>,
        }

        impl<'de, T> de::Visitor<'de> for Visitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Json<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON-encoded bulk string")
            }

            // `deserialize_any` routes a nil reply here, where serde's own
            // message would be `invalid type: Option`. The reply shape the
            // caller got and the type they need are both worth naming.
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(
                    "the reply is nil: a key that may be missing must be read as \
                     `Option<Json<T>>`, not `Json<T>`",
                ))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_none()
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(format!(
                    "expected a JSON-encoded bulk string, got the integer reply {v}"
                )))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(format!(
                    "expected a JSON-encoded bulk string, got the integer reply {v}"
                )))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(format!(
                    "expected a JSON-encoded bulk string, got the double reply {v}"
                )))
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(format!(
                    "expected a JSON-encoded bulk string, got the boolean reply {v}"
                )))
            }

            // `T: Deserialize<'de>` may borrow from the input, which data owned
            // by the deserializer does not outlive. Accepting these would force
            // `T: DeserializeOwned` and rule out borrowing types, so they are
            // diagnosed rather than supported.
            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(TRANSIENT_INPUT))
            }

            fn visit_string<E>(self, _v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(TRANSIENT_INPUT))
            }

            fn visit_bytes<E>(self, _v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(TRANSIENT_INPUT))
            }

            fn visit_byte_buf<E>(self, _v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(de::Error::custom(TRANSIENT_INPUT))
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value: T = serde_json::from_slice(v).map_err(|e| {
                    de::Error::custom(format!(
                        "Cannot deserialize from json (borrowed bytes): {}",
                        e
                    ))
                })?;
                Ok(Json(value))
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let value: T = serde_json::from_str(v).map_err(|e| {
                    de::Error::custom(format!(
                        "Cannot deserialize from json (borrowed str): {}",
                        e
                    ))
                })?;
                Ok(Json(value))
            }
        }

        deserializer.deserialize_any(Visitor {
            phantom: PhantomData,
        })
    }
}

impl<T> Serialize for Json<T>
where
    T: Serialize,
{
    /// A value that cannot be rendered as JSON fails the command: the error
    /// travels through the command builder's deferred error slot and surfaces
    /// from the awaited command, before anything is sent. An argument that
    /// cannot be written must never be replaced by an empty one, which would
    /// store an absent value under the caller's key and report success.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = serde_json::to_vec(&self.0)
            .map_err(|e| ser::Error::custom(format!("Cannot serialize to json: {e}")))?;
        serializer.serialize_bytes(&bytes)
    }
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
    use super::Json;
    use crate::{
        ClientError, Error,
        resp::{Command, FastPathCommandBuilder, RespBuf, cmd},
    };
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    struct Person {
        id: u32,
        name: String,
    }

    fn person() -> Person {
        Person {
            id: 12,
            name: "Foo".to_string(),
        }
    }

    /// A value whose `Serialize` impl always fails, standing in for any user type
    /// `serde_json` cannot render.
    struct FailingSerialize;
    impl Serialize for FailingSerialize {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    fn serialization_error_of(mut command: Command) -> Option<Error> {
        command.take_serialization_error()
    }

    #[test]
    fn a_failing_serialize_fails_the_command() {
        let mut command: Command = FastPathCommandBuilder::set("key", Json(&FailingSerialize));
        let error = command.take_serialization_error();
        assert!(
            matches!(&error, Some(Error::Client(ClientError::SerdeSerialize(m))) if m.contains("Cannot serialize to json")),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn a_serde_json_error_reaches_the_caller() {
        // A map with non-string keys is a real `serde_json` failure, not a
        // synthetic one.
        let map: BTreeMap<(u8, u8), u8> = BTreeMap::from([((1, 2), 3)]);
        let command: Command = FastPathCommandBuilder::set("key", Json(&map));
        assert!(matches!(
            serialization_error_of(command),
            Some(Error::Client(ClientError::SerdeSerialize(_)))
        ));
    }

    #[test]
    fn an_unserializable_value_is_never_written_as_an_empty_argument() {
        // A failing argument must leave the command incomplete and carrying the
        // error. An empty argument in the value's place would make Redis store
        // an absent value under the caller's key, and the call would report
        // success.
        let mut command: Command = FastPathCommandBuilder::set("key", Json(&FailingSerialize));
        assert!(command.take_serialization_error().is_some());
        assert_eq!(1, command.num_args());
        assert_eq!(Some(&b"key"[..]), command.get_arg(0).as_deref());
    }

    #[test]
    fn the_generic_builder_defers_the_same_error() {
        let command: Command = cmd("SET").key("key").arg(Json(&FailingSerialize)).into();
        assert!(matches!(
            serialization_error_of(command),
            Some(Error::Client(ClientError::SerdeSerialize(_)))
        ));
    }

    #[test]
    fn a_serializable_value_becomes_one_json_argument() {
        let mut command: Command = FastPathCommandBuilder::set("key", Json(&person()));
        assert!(command.take_serialization_error().is_none());
        assert_eq!(2, command.num_args());
        assert_eq!(
            Some(&br#"{"id":12,"name":"Foo"}"#[..]),
            command.get_arg(1).as_deref()
        );
    }

    #[test]
    fn a_borrowed_value_serializes_like_an_owned_one() {
        // `&T` is itself `Serialize`, so the wrapper covers both spellings.
        let person = person();
        let borrowed: Command = FastPathCommandBuilder::set("key", Json(&person));
        let owned: Command = FastPathCommandBuilder::set("key", Json(person.clone()));

        assert_eq!(borrowed.get_arg(1), owned.get_arg(1));
    }

    #[test]
    fn a_bulk_string_reply_deserializes() {
        let resp = RespBuf::from_slice(b"$22\r\n{\"id\":12,\"name\":\"Foo\"}\r\n");
        let Json(deserialized): Json<Person> = resp.to().unwrap();
        assert_eq!(person(), deserialized);
    }

    #[test]
    fn a_simple_string_reply_deserializes() {
        let resp = RespBuf::from_slice(b"+{\"id\":12,\"name\":\"Foo\"}\r\n");
        let Json(deserialized): Json<Person> = resp.to().unwrap();
        assert_eq!(person(), deserialized);
    }

    #[test]
    fn a_nil_reply_points_at_option_json() {
        let resp = RespBuf::from_slice(b"_\r\n");
        let error = resp.to::<Json<Person>>().unwrap_err();
        assert!(
            error.to_string().contains("Option<Json<T>>"),
            "unexpected error: {error}"
        );
        assert!(resp.to::<Option<Json<Person>>>().unwrap().is_none());
    }

    #[test]
    fn an_integer_reply_is_named_in_the_error() {
        let resp = RespBuf::from_slice(b":12\r\n");
        let error = resp.to::<Json<Person>>().unwrap_err();
        assert!(
            error.to_string().contains("integer"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn malformed_json_reports_the_serde_json_message() {
        let resp = RespBuf::from_slice(b"$3\r\nnot\r\n");
        let error = resp.to::<Json<Person>>().unwrap_err();
        assert!(
            error.to_string().contains("Cannot deserialize from json"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn into_inner_returns_the_wrapped_value() {
        assert_eq!(person(), Json(person()).into_inner());
    }
}
