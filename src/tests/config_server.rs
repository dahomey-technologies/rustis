//! The `config` tests that need a live Redis. The ones that need none stay in
//! `config.rs`.

use crate::{
    Result,
    client::{Client, Credentials, IntoConfig, ReconnectionConfig},
    commands::{ClientKillOptions, ConnectionCommands, FlushingMode, ServerCommands},
    tests::{get_default_host, get_default_port, get_test_client, log_try_init},
};
use serial_test::serial;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[tokio::test]
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
    let client = Client::connect(uri).await?;

    let client_info = client.client_info().await?;
    assert_eq!(1, client_info.db);

    Ok(())
}

#[tokio::test]
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

#[tokio::test]
#[serial]
async fn reconnection() -> Result<()> {
    log_try_init();
    let uri = format!("redis://{}:{}/1", get_default_host(), get_default_port());
    let client = Client::connect(uri.clone()).await?;

    // kill client connection from another client to force reconnection
    let client2 = Client::connect(uri).await?;
    let client_id = client.client_id().await?;
    client2
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    let client_info = client.client_info().retry_on_error(true).await?;
    assert_eq!(1, client_info.db);

    Ok(())
}

#[tokio::test]
#[serial]
async fn credentials_provider_authenticates() -> Result<()> {
    log_try_init();
    let admin = get_test_client().await?;
    admin.config_set(("requirepass", "pwd")).await?;

    let mut config =
        format!("redis://{}:{}", get_default_host(), get_default_port()).into_config()?;
    config.credentials_provider = Some(Arc::new(|| async {
        Ok(Credentials {
            username: None,
            password: "pwd".to_owned(),
        })
    }));
    let client = Client::connect(config).await?;
    client.client_id().await?;

    // reset password
    client.config_set(("requirepass", "")).await?;

    Ok(())
}

/// The whole point of a provider: a token that changed while the connection was
/// alive must be picked up by the reconnection handshake.
#[tokio::test]
#[serial]
async fn credentials_provider_refreshed_on_reconnect() -> Result<()> {
    log_try_init();
    let admin = get_test_client().await?;
    admin.config_set(("requirepass", "pwd1")).await?;

    let current_password = Arc::new(Mutex::new("pwd1".to_owned()));
    let calls = Arc::new(AtomicUsize::new(0));

    let provider_password = current_password.clone();
    let provider_calls = calls.clone();
    let mut config =
        format!("redis://{}:{}", get_default_host(), get_default_port()).into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    config.credentials_provider = Some(Arc::new(move || {
        let provider_password = provider_password.clone();
        let provider_calls = provider_calls.clone();
        async move {
            provider_calls.fetch_add(1, Ordering::SeqCst);
            let password = provider_password.lock().unwrap().clone();
            Ok(Credentials {
                username: None,
                password,
            })
        }
    }));

    let client = Client::connect(config).await?;
    let client_id = client.client_id().await?;
    assert_eq!(1, calls.load(Ordering::SeqCst));

    // rotate the server password behind the client's back, then kill its
    // connection: the reconnection must authenticate with the new one.
    let admin = Client::connect(format!(
        "redis://:pwd1@{}:{}",
        get_default_host(),
        get_default_port()
    ))
    .await?;
    admin.config_set(("requirepass", "pwd2")).await?;
    *current_password.lock().unwrap() = "pwd2".to_owned();
    admin
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    client.client_id().retry_on_error(true).await?;
    assert!(
        calls.load(Ordering::SeqCst) >= 2,
        "the provider was not consulted again on reconnection"
    );

    // reset password
    admin.config_set(("requirepass", "")).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn connect_timeout() -> Result<()> {
    log_try_init();
    let client = Client::connect("redis://127.0.0.1:6379?connect_timeout=10000").await?;
    client.flushdb(FlushingMode::Sync).await?;

    Ok(())
}
