use crate::{
    resp::{cmd}, tests::get_test_client, Result
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn send() -> Result<()> {
    let mut client = get_test_client().await?;

    client.send(cmd("PING")).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn forget() -> Result<()> {
    let mut client = get_test_client().await?;

    client.send_and_forget(cmd("PING"))?;
    client.send(cmd("PING")).await?;

    Ok(())
}