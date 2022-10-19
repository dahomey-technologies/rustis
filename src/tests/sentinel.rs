use crate::{tests::log_try_init, Client, ConnectionCommands, Result};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unreachable() -> Result<()> {
    log_try_init();
    let result = Client::connect("redis+sentinel://127.0.0.1:1234,127.0.0.1:5678/myservice").await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unknown_service() -> Result<()> {
    log_try_init();
    let result = Client::connect("redis+sentinel://127.0.0.1:26379/unknown").await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn connection() -> Result<()> {
    log_try_init();
    let client = Client::connect("redis+sentinel://127.0.0.1:26379/myservice").await?;
    client.hello(Default::default()).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn connection_with_failures() -> Result<()> {
    log_try_init();
    let client =
        Client::connect("redis+sentinel://127.0.0.1:1234,127.0.0.1:26379/myservice").await?;
    client.hello(Default::default()).await?;

    Ok(())
}
