use crate::{
    tests::get_test_client, ConnectionCommands, GenericCommands, HelloOptions, PingOptions, Result,
    ServerCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn echo() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.echo("hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_setname_getname() -> Result<()> {
    let client = get_test_client().await?;

    client.client_setname("Mike").await?;
    let client_name: Option<String> = client.client_getname().await?;
    assert_eq!(Some("Mike".to_string()), client_name);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hello_v2() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.hello(HelloOptions::new(2)).await?;
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

    let result = client.hello(HelloOptions::new(3)).await?;
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

    client.ping(PingOptions::default()).await?;
    let result: String = client.ping(PingOptions::default().message("value")).await?;
    assert_eq!("value", result);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn quit() -> Result<()> {
//     let client = Connection::connect(get_default_addr()).await?;

//     client.quit().await?;

//     // reconnection here
//     client.ping::<String, ()>(None).await?;

//     Ok(())
// }

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reset() -> Result<()> {
    let client = get_test_client().await?;

    client.reset().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn select() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(crate::FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    client.move_("key", 1).await?;
    client.select(1).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}
