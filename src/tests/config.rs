use crate::{
    tests::{get_default_host, get_default_port, get_test_client},
    Client, ClientKillOptions, ConnectionCommands, IntoConfig, Result, ServerCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn default_database() -> Result<()> {
    let database = 1;
    let uri = format!(
        "redis://{}:{}/{}",
        get_default_host(),
        get_default_port(),
        database
    );
    let client = Client::connect(uri).await?;

    let client_info = client.client_info().await?;
    assert_eq!(1, client_info.db);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn password() -> Result<()> {
    let client = get_test_client().await?;

    // set password
    client.config_set(("requirepass", "pwd")).await?;

    let uri = format!("redis://:pwd@{}:{}", get_default_host(), get_default_port());
    let client = Client::connect(uri).await?;

    // reset password
    client.config_set(("requirepass", "")).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reconnection() -> Result<()> {
    let client = get_test_client().await?;

    // set password
    client.config_set(("requirepass", "pwd")).await?;

    let uri = format!(
        "redis://:pwd@{}:{}/1",
        get_default_host(),
        get_default_port()
    );
    let client = Client::connect(uri.clone()).await?;

    // kill client connection from another client to force reconnection
    let client2 = Client::connect(uri).await?;
    let client_id = client.client_id().await?;
    client2
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    let client_info = client.client_info().await?;
    assert_eq!(1, client_info.db);

    // reset password
    client.config_set(("requirepass", "")).await?;

    Ok(())
}

#[test]
fn into_config() -> Result<()> {
    assert_eq!(
        "redis://127.0.0.1:6379",
        "127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1:6379",
        "127.0.0.1:6379".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1:6379",
        "127.0.0.1:6379".to_owned().into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1:6379",
        "redis://127.0.0.1:6379".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1:6379",
        "redis://127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://example.com:6379",
        "redis://example.com".into_config()?.to_string()
    );
    assert_eq!(
        "redis://:pwd@127.0.0.1:6379",
        "redis://:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:pwd@127.0.0.1:6379",
        "redis://username:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:pwd@127.0.0.1:6379/1",
        "redis://username:pwd@127.0.0.1/1"
            .into_config()?
            .to_string()
    );
    assert!("127.0.0.1:xyz".into_config().is_err());
    assert!("redis://127.0.0.1:xyz".into_config().is_err());
    assert!("redis://username@127.0.0.1".into_config().is_err());
    assert!("http://username@127.0.0.1".into_config().is_err());

    Ok(())
}
