use crate::{
    Error, RedisError, RedisErrorKind, Result,
    client::{BatchPreparedCommand, Client, ClientPreparedCommand},
    commands::{
        ClientCachingMode, ClientInfoAttribute, ClientKillOptions, ClientListOptions,
        ClientPauseMode, ClientReplyMode, ClientTrackingOptions, ClientTrackingStatus,
        ClientUnblockMode, ConnectionCommands, FlushingMode, GenericCommands, HelloOptions,
        ServerCommands, StringCommands,
    },
    network::spawn,
    resp::cmd,
    sleep,
    tests::{get_test_client, log_try_init},
};
use futures_util::StreamExt;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn auth() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.auth(Some("username"), "password").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::WrongPass,
            description: _
        }))
    ));

    let result = client.auth(None::<String>, "password").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_getredir() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let client1_id = client1.client_id().await?;

    client2
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default().redirect(client1_id),
        )
        .await?;

    let redir_id = client2.client_getredir().await?;
    assert_eq!(client1_id, redir_id);

    client2
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_help() -> Result<()> {
    let client = get_test_client().await?;
    let result: Vec<String> = client.client_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));
    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_id() -> Result<()> {
    let client = get_test_client().await?;

    let id = client.client_id().await?;
    assert!(id > 0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_info() -> Result<()> {
    let client = get_test_client().await?;

    let client_info = client.client_info().await?;
    log::debug!("client_info: {client_info:?}");
    assert!(client_info.id != 0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_kill() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let client_id = client1.client_id().await?;
    client2
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_list() -> Result<()> {
    let client = get_test_client().await?;

    let current_client_id = client.client_id().await?;

    let _result = client
        .client_list(ClientListOptions::default().client_id(current_client_id))
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_no_evict() -> Result<()> {
    let client = get_test_client().await?;

    client.client_no_evict(true).await?;
    client.client_no_evict(false).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_no_touch() -> Result<()> {
    let client = get_test_client().await?;

    client.client_no_touch(true).await?;
    client.client_no_touch(false).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_pause() -> Result<()> {
    let client = get_test_client().await?;

    client.client_pause(1000, ClientPauseMode::Write).await?;
    client.client_unpause().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_reply() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // single command
    client.client_reply(ClientReplyMode::Off).forget()?;
    client.set("key", "value").forget()?;
    client.client_reply(ClientReplyMode::On).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    // pipeline
    let mut pipeline = client.create_pipeline();
    pipeline.client_reply(ClientReplyMode::Off).forget();
    pipeline.set("key1", "value1").forget();
    pipeline.set("key2", "value2").forget();
    pipeline.set("key3", "value3").forget();
    pipeline.client_reply(ClientReplyMode::On).queue();
    pipeline.execute::<()>().await?;

    let values: Vec<String> = client.mget(["key1", "key2", "key3"]).await?;
    assert_eq!(3, values.len());
    assert_eq!("value1", values[0]);
    assert_eq!("value2", values[1]);
    assert_eq!("value3", values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_setname_getname() -> Result<()> {
    let client = get_test_client().await?;

    client.client_setname("Mike").await?;
    let client_name: Option<String> = client.client_getname().await?;
    assert_eq!(Some("Mike".to_string()), client_name);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_setinfo() -> Result<()> {
    let client = get_test_client().await?;
    client
        .client_setinfo(ClientInfoAttribute::LibName, "rustis")
        .await?;
    client
        .client_setinfo(ClientInfoAttribute::LibVer, "0.13.3")
        .await?;

    let attrs: String = client.send(cmd("CLIENT").arg("INFO"), None).await?.to()?;

    assert!(attrs.contains("lib-name=rustis lib-ver=0.13.3"));
    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_tracking() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=client1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=client2").await?;

    // prepare invalidations
    let mut invalidation_stream = client1.create_client_tracking_invalidation_stream()?;

    client1.set("key", "value").await?;

    client1
        .client_tracking(ClientTrackingStatus::On, ClientTrackingOptions::default())
        .await?;

    // Redis track our local caching
    let _value: String = client1.get("key").await?;

    client2.set("key", "new_value").await?;

    let keys_to_invalidate: Vec<String> = invalidation_stream.next().await.unwrap();
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!("key", keys_to_invalidate[0]);

    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    // optin
    client1
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default().optin(),
        )
        .await?;

    // Redis will not track our local caching because we are optin
    let _value: String = client1.get("key").await?;

    client2.set("key", "new_value2").await?;

    // Redis will track our local caching because of the client_caching command
    client1.client_caching(ClientCachingMode::Yes).await?;
    let _value: String = client1.get("key").await?;

    client2.set("key", "new_value3").await?;

    let keys_to_invalidate: Vec<String> = invalidation_stream.next().await.unwrap();
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!("key", keys_to_invalidate[0]);

    // broadcasting mode
    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    client1
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default().prefix("k").broadcasting(),
        )
        .await?;

    // Redis will track our local caching because key is in the prefix pattern we just set
    let _value: String = client1.get("key").await?;

    client2.set("key", "new_value4").await?;

    let keys_to_invalidate: Vec<String> = invalidation_stream.next().await.unwrap();
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!("key", keys_to_invalidate[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_tracking_info() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let tracking_info = client1.client_trackinginfo().await?;
    assert_eq!(1, tracking_info.flags.len());
    assert!(tracking_info.flags.contains(&"off".to_owned()));
    assert_eq!(-1, tracking_info.redirect);
    assert_eq!(0, tracking_info.prefixes.len());

    let client2_id = client2.client_id().await?;

    client1
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default().redirect(client2_id),
        )
        .await?;

    let tracking_info = client1.client_trackinginfo().await?;
    assert_eq!(1, tracking_info.flags.len());
    assert!(tracking_info.flags.contains(&"on".to_owned()));
    assert_eq!(client2_id, tracking_info.redirect);
    assert_eq!(0, tracking_info.prefixes.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn client_unblock() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let client_id = client1.client_id().await?;

    spawn(async move {
        let result = client1.wait(2, 10000).await;
        matches!(
            result,
            Err(Error::Redis(RedisError {
                kind: RedisErrorKind::Unblocked,
                description: _
            }))
        )
    });

    sleep(std::time::Duration::from_millis(100)).await;
    client2
        .client_unblock(client_id, ClientUnblockMode::Error)
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn echo() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.echo("hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn hello_v3() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.hello(HelloOptions::new(3)).await?;
    assert_eq!("redis", result.server);
    assert!(result.version.starts_with('8'));
    assert_eq!(3, result.proto);
    assert!(result.id > 0);
    assert_eq!("standalone", result.mode);
    assert_eq!("master", result.role);
    assert_eq!(5, result.modules.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ping() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.ping(None::<&str>).await?;
    assert_eq!("PONG", result);
    let result: String = client.ping(Some("value")).await?;
    assert_eq!("value", result);

    Ok(())
}

// #[cfg_attr(feature = "tokio-runtime", tokio::test)]
// #[cfg_attr(feature = "async-std-runtime", async_std::test)]
// #[serial]
// async fn quit() -> Result<()> {
//     let client = Connection::connect(get_default_addr()).await?;

//     client.quit().await?;

//     // reconnection here
//     client.ping::<String, ()>(None).await?;

//     Ok(())
// }

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn reset() -> Result<()> {
    let client = get_test_client().await?;

    client.reset().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn select() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    client.move_("key", 1).await?;
    client.select(1).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}
