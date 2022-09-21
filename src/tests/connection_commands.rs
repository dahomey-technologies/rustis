use crate::{
    tests::get_default_addr, ConnectionCommands, ConnectionMultiplexer, DatabaseCommandResult,
    Result, HelloOptions,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hello_v2() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let result = database.hello(HelloOptions::new(2)).send().await?;
    assert_eq!("redis", result.server);
    assert_eq!("7.0.3", result.version);
    assert_eq!(2, result.proto);
    assert!(result.id > 0);
    assert_eq!("standalone", result.mode);
    assert_eq!("master", result.role);
    assert_eq!(0, result.modules.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hello_v3() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let result = database.hello(HelloOptions::new(3)).send().await?;
    assert_eq!("redis", result.server);
    assert_eq!("7.0.3", result.version);
    assert_eq!(3, result.proto);
    assert!(result.id > 0);
    assert_eq!("standalone", result.mode);
    assert_eq!("master", result.role);
    assert_eq!(0, result.modules.len());

    Ok(())
}
