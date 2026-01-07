use serde::{
    Deserialize,
    de::{self},
    ser::Serialize,
};

/// Wrapper type that deserializes a Redis bulk string as JSON into a Rust value.
///
/// This is useful for retrieving structured data from Redis that was stored as JSON.
/// Typically used with commands like `GET`, `HGET`, or any command returning a bulk string.
///
/// # Example
/// ```rust
/// use rustis::{
///     client::Client,
///     commands::{FlushingMode, ServerCommands, StringCommands},
///     resp::{Json, JsonRef},
///     Result
/// };
///
/// #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
/// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
/// async fn main() -> Result<()> {
///     let client = Client::connect("127.0.0.1:6379").await?;
///     client.flushall(FlushingMode::Sync).await?;
///     let user1 = User { id: 12, name: "foo".to_string() };
///     client.set("user:123", JsonRef(&user1)).await;
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
    pub fn into_inner(self) -> T {
        self.0
    }
}

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
                formatter.write_str("Json")
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

/// Wrapper type that serializes a Rust value as JSON before sending it to Redis.
///
/// This is useful for storing structured data in Redis as a bulk string using JSON encoding.
/// Typically used with commands like `SET`, `HSET`, or any command that takes a bulk string.
///
/// # Example
/// ```rust
/// use rustis::{
///     client::Client,
///     commands::{FlushingMode, ServerCommands, StringCommands},
///     resp::{Json, JsonRef},
///     Result
/// };
///
/// #[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
/// struct User {
///     id: u32,
///     name: String,
/// }
///
/// #[cfg_attr(feature = "tokio-runtime", tokio::main)]
/// #[cfg_attr(feature = "async-std-runtime", async_std::main)]
/// async fn main() -> Result<()> {
///     let client = Client::connect("127.0.0.1:6379").await?;
///     client.flushall(FlushingMode::Sync).await?;
///     let user1 = User { id: 12, name: "foo".to_string() };
///     client.set("user:123", JsonRef(&user1)).await;
///     let Json(user2): Json<User> = client.get("user:123").await?;
///
///     assert_eq!(user1, user2);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct JsonRef<'a, T>(pub &'a T);

impl<'a, T> Serialize for JsonRef<'a, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Ok(bytes) = serde_json::to_vec(&self.0) {
            serializer.serialize_bytes(&bytes)
        } else {
            serializer.serialize_unit()
        }
    }
}
