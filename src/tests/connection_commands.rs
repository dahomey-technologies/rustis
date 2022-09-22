use crate::{
    tests::get_default_addr, Connection, ConnectionCommandResult, ConnectionCommands,
    GenericCommands, HelloOptions, Result, ServerCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hello_v2() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    let result = connection.hello(HelloOptions::new(2)).send().await?;
    assert_eq!("redis", result.server);
    assert!(result.version.starts_with("7"));
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
    let connection = Connection::connect(get_default_addr()).await?;

    let result = connection.hello(HelloOptions::new(3)).send().await?;
    assert_eq!("redis", result.server);
    assert!(result.version.starts_with("7"));
    assert_eq!(3, result.proto);
    assert!(result.id > 0);
    assert_eq!("standalone", result.mode);
    assert_eq!("master", result.role);
    assert_eq!(0, result.modules.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ping() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.ping::<String, ()>(None).send().await?;
    let result: String = connection.ping(Some("value")).send().await?;
    assert_eq!("value", result);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn quit() -> Result<()> {
//     let connection = Connection::connect(get_default_addr()).await?;

//     connection.quit().send().await?;

//     // reconnection here
//     connection.ping::<String, ()>(None).send().await?;

//     Ok(())
// }

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reset() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;

    connection.reset().send().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn select() -> Result<()> {
    let connection = Connection::connect(get_default_addr()).await?;
    connection
        .flushall(crate::FlushingMode::Sync)
        .send()
        .await?;

    connection.set("key", "value").send().await?;
    connection.move_("key", 1).send().await?;
    connection.select(1).send().await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value", value);

    Ok(())
}
