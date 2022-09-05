use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, HashCommands, Result, SetCommands,
};
use serial_test::serial;
use std::collections::{BTreeMap, HashMap, HashSet, BTreeSet};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn key_value_collection() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del("key").await?;
    let items =("field1", "value1");
    let len = database.hset("key", items).await?;
    assert_eq!(1, len);

    database.del("key").await?;
    let items = HashMap::from([("field1", "value1"), ("field2", "value2")]);
    let len = database.hset("key", items).await?;
    assert_eq!(2, len);

    database.del("key").await?;
    let items = BTreeMap::from([("field1", "value1"), ("field2", "value2")]);
    let len = database.hset("key", items).await?;
    assert_eq!(2, len);

    database.del("key").await?;
    let items = vec![("field1", "value1"), ("field2", "value2")];
    let len = database.hset("key", items).await?;
    assert_eq!(2, len);

    database.del("key").await?;
    let items = [("field1", "value1"), ("field2", "value2")];
    let len = database.hset("key", items).await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn set_collection() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.del("key").await?;
    let items = "member1";
    let len = database.sadd("key", items).await?;
    assert_eq!(1, len);

    database.del("key").await?;
    let items = ["member1", "member2"];
    let len = database.sadd("key", items).await?;
    assert_eq!(2, len);

    database.del("key").await?;
    let items = vec!["member1", "member2"];
    let len = database.sadd("key", items).await?;
    assert_eq!(2, len);

    database.del("key").await?;
    let items = HashSet::from(["member1", "member2"]);
    let len = database.sadd("key", items).await?;
    assert_eq!(2, len);

    database.del("key").await?;
    let items = BTreeSet::from(["member1", "member2"]);
    let len = database.sadd("key", items).await?;
    assert_eq!(2, len);

    Ok(())
}

