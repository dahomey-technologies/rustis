use crate::{tests::log_try_init, Client, ClusterCommands, Result};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn asking() -> Result<()> {
    log_try_init();
    let mut client = Client::connect("127.0.0.1:7000").await?;

    client.asking().await?;

    Ok(())
}
