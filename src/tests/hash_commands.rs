use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, HashCommands, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hget_hset() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    database.hset("key", &[("field", "value")]).await?;
    let value: String = database.hget("key", "field").await?;
    assert_eq!("value", value);

    Ok(())
}
