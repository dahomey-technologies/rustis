use std::collections::HashSet;

use crate::{tests::get_default_addr, ConnectionMultiplexer, GenericCommands, Result, SetCommands};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sadd_smembers() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let len = database.sadd("key", ["value1", "value2", "value3"]).await?;
    assert_eq!(3, len);

    let members: HashSet<String> = database.smembers("key").await?;
    assert_eq!(3, members.len()); 
    assert!(members.contains("value1")); 
    assert!(members.contains("value2")); 
    assert!(members.contains("value3")); 

    Ok(())
}
