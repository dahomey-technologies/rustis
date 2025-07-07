use crate::{
    commands::{FlushingMode, ServerCommands, StringCommands},
    resp::{Json, JsonRef},
    tests::get_test_client,
    Result,
};
use serde::{Deserialize, Serialize};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

    client.set("key", JsonRef(&person)).await?;
    let Json(person2): Json<Person> = client.get("key").await?;

    assert_eq!(person, person2);

    client.close().await?;

    Ok(())
}
