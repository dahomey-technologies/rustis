#[cfg(any(feature = "native-tls", feature = "rustls"))]
use crate::{commands::StringCommands, tests::get_tls_test_client, Result};
#[cfg(any(feature = "native-tls", feature = "rustls"))]
use serial_test::serial;

#[cfg(feature = "rustls")]
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tls() -> Result<()> {
    let client = get_tls_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg(feature = "native-tls")]
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tls() -> Result<()> {
    let client = get_tls_test_client().await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}
