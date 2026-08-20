//! Checks the key-arity rule against real third-party types, from outside the
//! crate.
//!
//! The rule is stated on the `resp` page: a command argument is `impl Serialize`
//! rather than a trait of this crate's own, because Rust's orphan rule would
//! leave a user unable to implement such a trait for `uuid::Uuid` or
//! `serde_json::Value` — and because a trait could not answer for `Value`, whose
//! argument count varies by variant. What is checked instead is the count.
//!
//! That claim is about types this crate has never heard of, so a stand-in
//! written here would not test it: the types below come from their own crates,
//! as dev dependencies, and neither `uuid` nor `serde_json` has any `impl` from
//! **rustis**. The file lives in `tests/` so the crate is reached the way a
//! downstream user reaches it, through its public surface only.
//!
//! Unlike `tests/public_api.rs`, these bodies run: what an arity failure does is
//! the point, and a compile alone would not show it.

use rustis::{
    ClientError, ErrorKind, Result,
    client::Client,
    commands::{GenericCommands, HashCommands, StringCommands},
};
use serde_json::{Value, json};
use uuid::Uuid;

async fn connect() -> Result<Client> {
    Client::connect("127.0.0.1:6379").await
}

/// Whether an error is the arity failure, raised for `command`.
fn is_arity_error(error: &rustis::Error, command: &str) -> bool {
    matches!(
        error.kind(),
        ErrorKind::Client(ClientError::InvalidKeyArity { command: named, .. }) if named == command
    )
}

/// A `Uuid` is a valid key with no `impl` from this crate: it serializes to one
/// argument, which is the whole requirement. A marker trait could not have
/// accepted it — neither the trait nor `Uuid` would belong to the user's crate,
/// so the `impl` is impossible to write anywhere.
#[tokio::test]
async fn a_uuid_is_a_valid_key() -> Result<()> {
    let client = connect().await?;
    let key = Uuid::new_v4();

    client.set(key, "value").await?;
    let value: String = client.get(key).await?;
    assert_eq!("value", value);

    // The key on the wire is the `Uuid`'s own string form, one argument, so a
    // second client reaching the same entry needs no agreement with us.
    let value: String = client.get(key.to_string()).await?;
    assert_eq!("value", value);

    client.del(key).await?;
    client.close().await?;

    Ok(())
}

/// `serde_json::Value` is the type that settles the design: one type, several
/// argument counts. `String` and `Number` write one and are keys; `Null` writes
/// none and `Array` writes one per element, so neither is. No trait `impl` could
/// have told these apart, the count being a property of the value.
#[tokio::test]
async fn a_json_value_is_a_key_or_not_depending_on_its_variant() -> Result<()> {
    let client = connect().await?;

    for accepted in [json!("a_string_key"), json!(42)] {
        client.set(accepted.clone(), "value").await?;
        let value: String = client.get(accepted.clone()).await?;
        assert_eq!("value", value, "{accepted} writes one argument");
        client.del(accepted).await?;
    }

    for rejected in [Value::Null, json!(["a", "b"]), json!({"tenant": "acme"})] {
        let result: Result<String> = client.get(rejected.clone()).await;
        let error = result.unwrap_err();
        assert!(
            is_arity_error(&error, "GET"),
            "{rejected} is not a single key, got {error:?}"
        );
    }

    client.close().await?;

    Ok(())
}

/// Values are not checked, and must not be: a JSON object as a value is the same
/// flattening that makes `HSET` take a struct. The arity rule applies to keys
/// alone.
#[tokio::test]
async fn a_json_object_is_a_valid_set_of_field_values() -> Result<()> {
    let client = connect().await?;
    let key = Uuid::new_v4();

    client
        .hset(key, json!({"tenant": "acme", "id": "42"}))
        .await?;

    let tenant: String = client.hget(key, "tenant").await?;
    assert_eq!("acme", tenant);

    client.del(key).await?;
    client.close().await?;

    Ok(())
}

/// A multi-key command takes the collection, and an empty one is refused: it
/// would send a command with no key, and so with no hash slot to route on.
#[tokio::test]
async fn a_collection_of_foreign_keys_is_accepted_but_not_an_empty_one() -> Result<()> {
    let client = connect().await?;
    let keys = [Uuid::new_v4(), Uuid::new_v4()];

    for key in keys {
        client.set(key, "value").await?;
    }
    assert_eq!(2, client.del(keys).await?);

    let error = client.del(Vec::<Uuid>::new()).await.unwrap_err();
    assert!(
        is_arity_error(&error, "DEL"),
        "an empty key list is refused, got {error:?}"
    );

    client.close().await?;

    Ok(())
}
