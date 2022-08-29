use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, Result, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn del() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key", "value").await?;

    let deleted = database.del("key").await?;
    assert_eq!(1, deleted);

    let deleted = database.del("key").await?;
    assert_eq!(0, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn del_multiple() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key1", "value1").await?;
    database.set("key2", "value2").await?;
    database.set("key3", "value3").await?;

    let deleted = database.del("key1").await?;
    assert_eq!(1, deleted);

    let deleted = database.del(["key1", "key2", "key3"]).await?;
    assert_eq!(2, deleted);

    Ok(())
}
