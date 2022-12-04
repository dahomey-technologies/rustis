use crate::{
    client::PooledClientManager, commands::StringCommands, tests::get_default_addr, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pooled_client_manager() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr())?;
    let pool = crate::bb8::Pool::builder().build(manager).await?;
    let mut client = pool.get().await.unwrap();

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}
