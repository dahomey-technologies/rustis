use crate::{
    Result,
    commands::{
        FlushingMode, GenericCommands, JsonCommands, JsonGetOptions, ServerCommands, StringCommands,
    },
    resp::Json,
    tests::get_test_client,
};
use serde::{Deserialize, Serialize};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_set_json() -> Result<()> {
    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
    pub struct Person {
        pub id: u32,
        pub name: String,
    }

    let person = Person {
        id: 12,
        name: "Foo".to_string(),
    };

    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", Json(&person)).await?;
    let Json(person2): Json<Person> = client.get("key").await?;

    assert_eq!(person, person2);

    // An owned value round-trips the same way, and `into_inner` unwraps what
    // destructuring gives above.
    client.set("key2", Json(person.clone())).await?;
    let person3: Json<Person> = client.get("key2").await?;

    assert_eq!(person, person3.into_inner());

    // A missing key is a nil reply, which is not a JSON document.
    assert!(client.get::<Json<Person>>("missing").await.is_err());
    let missing: Option<Json<Person>> = client.get("missing").await?;
    assert!(missing.is_none());

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn a_value_that_cannot_be_serialized_fails_the_command() -> Result<()> {
    struct FailingSerialize;
    impl Serialize for FailingSerialize {
        fn serialize<S: serde::Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("boom"))
        }
    }

    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // The command must fail rather than store an empty value under the key.
    assert!(client.set("key", Json(&FailingSerialize)).await.is_err());
    assert_eq!(0, client.exists("key").await?);

    client.close().await?;

    Ok(())
}

/// The wrapper reads back what the JSON feature itself stored, not only what
/// `SET` stored.
#[tokio::test]
#[serial]
async fn json_get_into_the_json_wrapper() -> Result<()> {
    #[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
    pub struct Person {
        pub id: u32,
        pub name: String,
    }

    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set("key", "$", r#"{"id":12,"name":"Foo"}"#, None)
        .await?;
    let Json(person): Json<Person> = client.json_get("key", JsonGetOptions::default()).await?;

    assert_eq!(
        Person {
            id: 12,
            name: "Foo".to_string()
        },
        person
    );

    client.close().await?;

    Ok(())
}
