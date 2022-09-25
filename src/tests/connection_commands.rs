use crate::{
    tests::get_test_client, ConnectionCommandResult, ConnectionCommands, GenericCommands,
    HelloOptions, Result, ServerCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hello_v2() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.hello(HelloOptions::new(2)).send().await?;
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
    let client = get_test_client().await?;

    let result = client.hello(HelloOptions::new(3)).send().await?;
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
    let client = get_test_client().await?;

    client.ping::<String, ()>(None).send().await?;
    let result: String = client.ping(Some("value")).send().await?;
    assert_eq!("value", result);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn quit() -> Result<()> {
//     let client = Connection::connect(get_default_addr()).await?;

//     client.quit().send().await?;

//     // reconnection here
//     client.ping::<String, ()>(None).send().await?;

//     Ok(())
// }

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reset() -> Result<()> {
    let client = get_test_client().await?;

    client.reset().send().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn select() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(crate::FlushingMode::Sync).send().await?;

    client.set("key", "value").send().await?;
    client.move_("key", 1).send().await?;
    client.select(1).send().await?;
    let value: String = client.get("key").send().await?;
    assert_eq!("value", value);

    Ok(())
}
