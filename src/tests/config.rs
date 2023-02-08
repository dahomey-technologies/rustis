use crate::{
    client::{Client, IntoConfig},
    commands::{ClientKillOptions, ConnectionCommands, ServerCommands, FlushingMode},
    tests::{get_default_host, get_default_port, get_test_client, log_try_init},
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn default_database() -> Result<()> {
    log_try_init();
    let database = 1;
    let uri = format!(
        "redis://{}:{}/{}",
        get_default_host(),
        get_default_port(),
        database
    );
    let mut client = Client::connect(uri).await?;

    let client_info = client.client_info().await?;
    assert_eq!(1, client_info.db);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn password() -> Result<()> {
    let mut client = get_test_client().await?;

    // set password
    client.config_set(("requirepass", "pwd")).await?;

    let uri = format!("redis://:pwd@{}:{}", get_default_host(), get_default_port());
    let mut client = Client::connect(uri).await?;

    // reset password
    client.config_set(("requirepass", "")).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reconnection() -> Result<()> {
    let uri = format!(
        "redis://{}:{}/1",
        get_default_host(),
        get_default_port()
    );
    let mut client = Client::connect(uri.clone()).await?;

    // kill client connection from another client to force reconnection
    let mut client2 = Client::connect(uri).await?;
    let client_id = client.client_id().await?;
    client2
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    let client_info = client.client_info().await?;
    assert_eq!(1, client_info.db);

    Ok(())
}

#[test]
fn into_config() -> Result<()> {
    assert_eq!(
        "redis://127.0.0.1",
        "127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1",
        "127.0.0.1:6379".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1",
        "127.0.0.1".to_owned().into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1",
        "redis://127.0.0.1:6379".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1",
        "redis://127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://example.com",
        "redis://example.com".into_config()?.to_string()
    );
    assert_eq!(
        "redis://:pwd@127.0.0.1",
        "redis://:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:pwd@127.0.0.1",
        "redis://username:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:pwd@127.0.0.1/1",
        "redis://username:pwd@127.0.0.1/1"
            .into_config()?
            .to_string()
    );    
    #[cfg(feature = "tls")]
    assert_eq!(
        "rediss://username:pwd@127.0.0.1/1",
        "rediss://username:pwd@127.0.0.1/1"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?connect_timeout=100",
        "redis://127.0.0.1?connect_timeout=100"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1",
        "redis://127.0.0.1?auto_resubscribe=true"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?auto_resubscribe=false",
        "redis://127.0.0.1?auto_resubscribe=false"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1",
        "redis://127.0.0.1?auto_remonitor=true"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?auto_remonitor=false",
        "redis://127.0.0.1?auto_remonitor=false"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?connection_name=myclient",
        "redis://127.0.0.1?connection_name=myclient"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?keep_alive=30000",
        "redis://127.0.0.1?keep_alive=30000"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?no_delay=false",
        "redis://127.0.0.1?no_delay=false"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?max_command_attempts=4",
        "redis://127.0.0.1?max_command_attempts=4"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?retry_on_error=true",
        "redis://127.0.0.1?retry_on_error=true"
            .into_config()?
            .to_string()
    );
    assert_eq!(
        "redis+sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice/1",
        "redis+sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice/1"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice",
        "redis+sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://username:pwd@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice",
        "redis+sentinel://username:pwd@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://:pwd@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice",
        "redis+sentinel://:pwd@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice",
        "redis+sentinel://127.0.0.1:6379/myservice"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=100&sentinel_username=foo&sentinel_password=bar",
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=100&sentinel_username=foo&sentinel_password=bar"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice?sentinel_username=foo&sentinel_password=bar",
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=250&sentinel_username=foo&sentinel_password=bar"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice?connect_timeout=100&wait_between_failures=100&sentinel_username=foo&sentinel_password=bar",
        "redis+sentinel://127.0.0.1:6379/myservice?connect_timeout=100&wait_between_failures=100&sentinel_username=foo&sentinel_password=bar"
            .into_config()?
            .to_string()
    );

    assert!("127.0.0.1:xyz".into_config().is_err());
    assert!("redis://127.0.0.1:xyz".into_config().is_err());
    assert!("redis://username@127.0.0.1".into_config().is_err());
    assert!("http://username@127.0.0.1".into_config().is_err());
    assert!(
        "redis+sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381"
            .into_config()
            .is_err()
    );
    assert!("redis://127.0.0.1?param".into_config().is_err());
    assert!("redis://127.0.0.1?param=value".into_config().is_ok());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn connect_timeout() -> Result<()> {
    log_try_init();
    let mut client = Client::connect("redis://127.0.0.1:6379?connect_timeout=10000").await?;
    client.flushdb(FlushingMode::Sync).await?;

    Ok(())
}
