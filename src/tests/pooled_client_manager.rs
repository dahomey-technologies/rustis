use crate::{
    tests::get_default_addr, ClientCommandResult, PooledClientManager, Result, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pooled_client_manager() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr());
    let pool = crate::bb8::Pool::builder().build(manager).await?;
    let client = pool.get().await.unwrap();

    client.set("key", "value").send().await?;
    let value: String = client.get("key").send().await?;
    assert_eq!("value", value);

    Ok(())
}
