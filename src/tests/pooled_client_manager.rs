use crate::{
    Result, client::PooledClientManager, commands::StringCommands, tests::get_default_addr,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn pooled_client_manager() -> Result<()> {
    let manager = PooledClientManager::new(get_default_addr())?;
    let pool = crate::bb8::Pool::builder().build(manager).await?;
    let client = pool.get().await.unwrap();

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}
