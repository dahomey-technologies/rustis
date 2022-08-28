use crate::{cmd, ConnectionMultiplexer, Result, StringCommands};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn database() -> Result<()> {
    let connection = ConnectionMultiplexer::connect().await?;
    let database0 = connection.get_database(0);
    database0.set("key", "value0").await?;

    assert_eq!("value0", &database0.get::<_, String>("key").await?);

    let database1 = connection.get_database(1);
    database1.set("key", "value1").await?;

    assert_eq!("value0", &database0.get::<_, String>("key").await?);
    assert_eq!("value1", &database1.get::<_, String>("key").await?);

    let database0 = connection.get_database(0);
    database0.set("key", "value00").await?;

    assert_eq!("value00", &database0.get::<_, String>("key").await?);
    assert_eq!("value1", &database1.get::<_, String>("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn send() -> Result<()> {
    let connection = ConnectionMultiplexer::connect().await?;
    let database = connection.get_default_database();

    database.set("key1", "value1").await?;
    database.set("key2", "value2").await?;
    database.set("key3", "value3").await?;
    database.set("key4", "value4").await?;
    database.set("key5", "value5").await?;

    let values: Vec<String> = database
        .send(
            cmd("MGET")
                .arg("key1")
                .arg("key2")
                .arg("key3")
                .arg("key4")
                .arg("key5"),
        )
        .await?
        .into()?;
    assert_eq!(5, values.len());
    assert_eq!("value1", values[0]);
    assert_eq!("value2", values[1]);
    assert_eq!("value3", values[2]);
    assert_eq!("value4", values[3]);
    assert_eq!("value5", values[4]);

    Ok(())
}
