use crate::{
    ErrorKind, RedisError, RedisErrorKind, Result,
    client::{BatchPreparedCommand, Client, ClientPreparedCommand, ReconnectionConfig},
    commands::{
        ClientCachingMode, ClientInfoAttribute, ClientKillOptions, ClientListOptions,
        ClientPauseMode, ClientReplyMode, ClientTrackingOptions, ClientTrackingStatus, ClientType,
        ClientUnblockMode, ConnectionCommands, FlushingMode, GenericCommands, HelloOptions,
        ServerCommands, StringCommands,
    },
    network::spawn,
    resp::{BulkString, cmd},
    sleep,
    tests::{get_default_config, get_test_client, get_test_client_with_config, log_try_init},
};
use futures_util::StreamExt;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn auth() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.auth(Some("username"), "password").await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::WrongPass,
            description: _
        })
    ));

    let result = client.auth(None::<String>, "password").await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    Ok(())
}

#[tokio::test]
#[serial]
/// `CLIENT GETREDIR` reports where invalidations are being sent: `-1` when tracking
/// is off, `0` when it is on without a redirection. Those are the only two answers
/// rustis can produce — `REDIRECT` is deliberately not offered, see
/// [`ClientTrackingOptions`].
async fn client_getredir() -> Result<()> {
    let client = get_test_client().await?;

    client
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;
    assert_eq!(-1, client.client_getredir().await?);

    client
        .client_tracking(ClientTrackingStatus::On, ClientTrackingOptions::default())
        .await?;
    assert_eq!(0, client.client_getredir().await?);

    client
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_help() -> Result<()> {
    let client = get_test_client().await?;
    let result: Vec<String> = client.client_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn client_id() -> Result<()> {
    let client = get_test_client().await?;

    let id = client.client_id().await?;
    assert!(id > 0);

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_info() -> Result<()> {
    let client = get_test_client().await?;

    let client_info = client.client_info().await?;
    tracing::debug!("client_info: {client_info:?}");
    assert!(client_info.id != 0);

    Ok(())
}

#[tokio::test]
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

/// Every filter of `CLIENT KILL <option> <value> [...]` as `CLIENT HELP` prints
/// it: ADDR, LADDR, TYPE, USER, SKIPME, ID, MAXAGE. Each filter is paired with an
/// address that matches no connection, so the server parses the whole clause but
/// kills nothing.
#[tokio::test]
#[serial]
async fn client_kill_options() -> Result<()> {
    let client = get_test_client().await?;

    let unmatched = || ClientKillOptions::default().addr("1.2.3.4", 1);

    assert_eq!(0, client.client_kill(unmatched()).await?);
    assert_eq!(
        0,
        client.client_kill(unmatched().laddr("1.2.3.4", 1)).await?
    );
    assert_eq!(
        0,
        client
            .client_kill(unmatched().client_type(ClientType::PubSub))
            .await?
    );
    assert_eq!(0, client.client_kill(unmatched().user("default")).await?);
    assert_eq!(0, client.client_kill(unmatched().skip_me(false)).await?);
    assert_eq!(0, client.client_kill(unmatched().skip_me(true)).await?);
    assert_eq!(0, client.client_kill(unmatched().max_age(100_000)).await?);
    assert_eq!(0, client.client_kill(unmatched().id(999_999_999)).await?);

    Ok(())
}

/// `CLIENT LIST [TYPE normal|master|replica|pubsub] [ID client-id [client-id ...]]`.
#[tokio::test]
#[serial]
async fn client_list_options() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let id1 = client1.client_id().await?;
    let id2 = client2.client_id().await?;

    let result = client1
        .client_list(ClientListOptions::default().client_type(ClientType::Normal))
        .await?;
    let ids: Vec<i64> = result.client_infos.iter().map(|info| info.id).collect();
    assert!(ids.contains(&id1));
    assert!(ids.contains(&id2));

    let result = client1
        .client_list(ClientListOptions::default().client_ids([id1, id2]))
        .await?;
    let mut ids: Vec<i64> = result.client_infos.iter().map(|info| info.id).collect();
    ids.sort_unstable();
    let mut expected = vec![id1, id2];
    expected.sort_unstable();
    assert_eq!(expected, ids);

    Ok(())
}

/// `CLIENT TRACKING ON [OPTOUT] [NOLOOP]`, read back through `CLIENT TRACKINGINFO`
/// whose `flags` field is the server's own name for each mode.
#[tokio::test]
#[serial]
async fn client_tracking_optout_and_noloop() -> Result<()> {
    let client = get_test_client().await?;

    client
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default().optout().noloop(),
        )
        .await?;

    let tracking_info = client.client_trackinginfo().await?;
    assert!(tracking_info.flags.contains(&"on".to_owned()));
    assert!(tracking_info.flags.contains(&"optout".to_owned()));
    assert!(tracking_info.flags.contains(&"noloop".to_owned()));

    client
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    Ok(())
}

/// `HELLO [protover [AUTH username password] [SETNAME clientname]]`.
#[tokio::test]
#[serial]
async fn hello_set_name() -> Result<()> {
    let client = get_test_client().await?;

    client
        .hello(HelloOptions::new(3).set_name("hello_name"))
        .await?;

    let name: String = client.client_getname().await?;
    assert_eq!("hello_name", name);

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_list() -> Result<()> {
    let client = get_test_client().await?;

    let current_client_id = client.client_id().await?;

    let _result = client
        .client_list(ClientListOptions::default().client_id(current_client_id))
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_no_evict() -> Result<()> {
    let client = get_test_client().await?;

    client.client_no_evict(true).await?;
    client.client_no_evict(false).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_no_touch() -> Result<()> {
    let client = get_test_client().await?;

    client.client_no_touch(true).await?;
    client.client_no_touch(false).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_pause() -> Result<()> {
    let client = get_test_client().await?;

    client.client_pause(1000, ClientPauseMode::Write).await?;
    client.client_unpause().await?;

    Ok(())
}

#[tokio::test]
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

#[tokio::test]
#[serial]
async fn client_reply_skip() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.client_reply(ClientReplyMode::Skip).forget()?;
    client.set("skip_key", "skip_value").forget()?;
    client.client_reply(ClientReplyMode::On).await?;
    let value: String = client.get("skip_key").await?;
    assert_eq!("skip_value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_setname_getname() -> Result<()> {
    let client = get_test_client().await?;

    client.client_setname("Mike").await?;
    let client_name: Option<String> = client.client_getname().await?;
    assert_eq!(Some("Mike".to_string()), client_name);

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_setinfo() -> Result<()> {
    let client = get_test_client().await?;
    client
        .client_setinfo(ClientInfoAttribute::LibName, "rustis")
        .await?;
    client
        .client_setinfo(ClientInfoAttribute::LibVer, "0.13.3")
        .await?;

    let attrs: String = client.send(cmd("CLIENT").arg("INFO"), None).await?;

    assert!(attrs.contains("lib-name=rustis lib-ver=0.13.3"));
    Ok(())
}

#[tokio::test]
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

    let keys_to_invalidate: Vec<BulkString> = invalidation_stream.next().await.unwrap();
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!(b"key", keys_to_invalidate[0].as_bytes());

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

    let keys_to_invalidate: Vec<BulkString> = invalidation_stream.next().await.unwrap();
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!(b"key", keys_to_invalidate[0].as_bytes());

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

    let keys_to_invalidate: Vec<BulkString> = invalidation_stream.next().await.unwrap();
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!(b"key", keys_to_invalidate[0].as_bytes());

    Ok(())
}

/// Redis keys are binary-safe, so an invalidation can name a key that is not
/// valid UTF-8. Such an event must still reach the consumer, and above all must
/// not end the stream: once ended, no invalidation ever arrives again and every
/// cache built on top of it silently serves stale data for good.
#[tokio::test]
#[serial]
async fn client_tracking_invalidation_survives_a_binary_key() -> Result<()> {
    log_try_init();
    let client1 = Client::connect("redis://127.0.0.1?connection_name=binary1").await?;
    let client2 = Client::connect("redis://127.0.0.1?connection_name=binary2").await?;

    let mut invalidation_stream = client1.create_client_tracking_invalidation_stream()?;

    let binary_key = BulkString::new(vec![0xffu8, 0xfe]);
    client1.set(binary_key.clone(), "value").await?;
    client1.set("text_key", "value").await?;

    client1
        .client_tracking(ClientTrackingStatus::On, ClientTrackingOptions::default())
        .await?;

    // Have both keys tracked for this connection.
    let _value: String = client1.get(binary_key.clone()).await?;
    let _value: String = client1.get("text_key").await?;

    client2.set(binary_key.clone(), "new_value").await?;

    let keys_to_invalidate: Vec<BulkString> = invalidation_stream
        .next()
        .await
        .expect("a binary key must not be reported as the end of the stream");
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!(binary_key.as_bytes(), keys_to_invalidate[0].as_bytes());

    // The stream must still be alive: this second invalidation is the assertion
    // that the binary one did not terminate it.
    client2.set("text_key", "new_value").await?;

    let keys_to_invalidate: Vec<BulkString> = invalidation_stream
        .next()
        .await
        .expect("later invalidations must still be delivered");
    assert_eq!(1, keys_to_invalidate.len());
    assert_eq!(b"text_key", keys_to_invalidate[0].as_bytes());

    client1
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_tracking_info() -> Result<()> {
    let client = get_test_client().await?;

    let tracking_info = client.client_trackinginfo().await?;
    assert_eq!(1, tracking_info.flags.len());
    assert!(tracking_info.flags.contains(&"off".to_owned()));
    assert_eq!(-1, tracking_info.redirect);
    assert_eq!(0, tracking_info.prefixes.len());

    // `PREFIX` is only accepted alongside `BCAST`.
    client
        .client_tracking(
            ClientTrackingStatus::On,
            ClientTrackingOptions::default()
                .broadcasting()
                .prefix("tracking_info_prefix"),
        )
        .await?;

    let tracking_info = client.client_trackinginfo().await?;
    assert!(tracking_info.flags.contains(&"on".to_owned()));
    assert!(tracking_info.flags.contains(&"bcast".to_owned()));
    // No redirection is reachable through the API — see `ClientTrackingOptions` —
    // so `0` is the only value this field can carry while tracking is on.
    assert_eq!(0, tracking_info.redirect);
    assert_eq!(
        vec!["tracking_info_prefix".to_owned()],
        tracking_info.prefixes
    );

    client
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn client_unblock() -> Result<()> {
    let client1 = get_test_client().await?;
    let client2 = get_test_client().await?;

    let client_id = client1.client_id().await?;

    spawn(async move {
        let result = client1.wait(2, 10000).await;
        matches!(
            result.unwrap_err().kind(),
            ErrorKind::Redis(RedisError {
                kind: RedisErrorKind::Unblocked,
                description: _
            })
        )
    });

    sleep(std::time::Duration::from_millis(100)).await;
    client2
        .client_unblock(client_id, ClientUnblockMode::Error)
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn echo() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.echo("hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

#[tokio::test]
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

#[tokio::test]
#[serial]
async fn ping() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.ping(()).await?;
    assert_eq!("PONG", result);
    let result: String = client.ping(Some("value")).await?;
    assert_eq!("value", result);

    Ok(())
}

// #[tokio::test]
// #[serial]
// async fn quit() -> Result<()> {
//     let client = Connection::connect(get_default_addr()).await?;

//     client.quit().await?;

//     // reconnection here
//     client.ping::<String, ()>(None).await?;

//     Ok(())
// }

#[tokio::test]
#[serial]
async fn reset() -> Result<()> {
    let client = get_test_client().await?;

    client.reset().await?;

    Ok(())
}

#[tokio::test]
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

/// Connects a client whose reconnection delay is short enough for a test, kills
/// its connection from the inside, and returns once the new one is up.
///
/// Every probe below shares the same shape: set a piece of connection-attached
/// state, lose the socket, then ask the server what that state is now.
async fn reconnecting_test_client() -> Result<Client> {
    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    get_test_client_with_config(config).await
}

async fn force_reconnection(client: &Client) -> Result<()> {
    let mut on_reconnect = client.on_reconnect();
    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;
    on_reconnect
        .recv()
        .await
        .expect("the client should have reconnected");
    Ok(())
}

/// The database a caller selected is connection state: a fresh socket starts on
/// database 0, so every key the caller reads afterwards silently comes from the
/// wrong database.
#[tokio::test]
#[serial]
async fn selected_database_is_restored_after_reconnect() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client.select(7).await?;
    client.flushdb(FlushingMode::Sync).await?;
    client.set("db_probe", "value").await?;

    force_reconnection(&client).await?;

    let value: Option<String> = client.get("db_probe").await?;
    assert_eq!(
        Some("value".to_owned()),
        value,
        "reads after a reconnection must still target the selected database"
    );

    client.flushdb(FlushingMode::Sync).await?;
    Ok(())
}

/// When the config names a database and the caller has since selected another,
/// the one the caller is on must win — and the config's must not be emitted at
/// all, since the replay would overwrite it a round-trip later.
#[tokio::test]
#[serial]
async fn a_runtime_select_wins_over_the_configured_database() -> Result<()> {
    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    config.database = 1;
    let client = get_test_client_with_config(config).await?;

    client.select(7).await?;

    force_reconnection(&client).await?;

    let info: String = client.send(cmd("CLIENT").arg("INFO"), None).await?;
    let db = info
        .split(' ')
        .find_map(|field| field.strip_prefix("db="))
        .unwrap_or_default();
    assert_eq!(
        "7", db,
        "the database the caller selected must win over the configured one"
    );

    Ok(())
}

/// A name set at runtime is what identifies the connection in `CLIENT LIST`,
/// `SLOWLOG` and the server logs. Losing it on reconnect makes the connection
/// anonymous exactly when an operator is looking for it.
#[tokio::test]
#[serial]
async fn connection_name_is_restored_after_reconnect() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client.client_setname("name_probe").await?;

    force_reconnection(&client).await?;

    let name: Option<String> = client.client_getname().await?;
    assert_eq!(
        Some("name_probe".to_owned()),
        name,
        "a name set at runtime must survive a reconnection"
    );

    Ok(())
}

/// Same argument as the connection name, for the library identification the
/// server reports in `CLIENT INFO`.
#[tokio::test]
#[serial]
async fn client_setinfo_is_restored_after_reconnect() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client
        .client_setinfo(ClientInfoAttribute::LibName, "rustis-probe")
        .await?;
    client
        .client_setinfo(ClientInfoAttribute::LibVer, "9.9.9")
        .await?;

    force_reconnection(&client).await?;

    let info: String = client.send(cmd("CLIENT").arg("INFO"), None).await?;
    assert!(
        info.contains("lib-name=rustis-probe lib-ver=9.9.9"),
        "the library identification must survive a reconnection, got: {info}"
    );

    Ok(())
}

/// `CLIENT NO-EVICT` and `CLIENT NO-TOUCH` protect a connection from eviction
/// and from bumping key idle times. Both are silently disarmed by a reconnection,
/// and nothing tells the caller.
#[tokio::test]
#[serial]
async fn no_evict_and_no_touch_are_restored_after_reconnect() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client.client_no_evict(true).await?;
    client.client_no_touch(true).await?;

    force_reconnection(&client).await?;

    let info: String = client.send(cmd("CLIENT").arg("INFO"), None).await?;
    let flags = info
        .split(' ')
        .find_map(|field| field.strip_prefix("flags="))
        .unwrap_or_default();

    assert!(
        flags.contains('e'),
        "NO-EVICT must survive a reconnection, flags: {flags}"
    );
    assert!(
        flags.contains('T'),
        "NO-TOUCH must survive a reconnection, flags: {flags}"
    );

    Ok(())
}

/// Client-side caching enabled directly, without going through `Cache`: the
/// invalidation stream survives the reconnection, so a caller keeps consuming a
/// channel the server no longer feeds.
#[tokio::test]
#[serial]
async fn client_tracking_is_restored_after_reconnect() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client
        .client_tracking(ClientTrackingStatus::On, ClientTrackingOptions::default())
        .await?;

    force_reconnection(&client).await?;

    let tracking_info = client.client_trackinginfo().await?;
    assert!(
        !tracking_info.flags.iter().any(|flag| flag == "off"),
        "tracking must be re-armed after a reconnection, flags: {:?}",
        tracking_info.flags
    );

    client
        .client_tracking(ClientTrackingStatus::Off, ClientTrackingOptions::default())
        .await?;
    Ok(())
}

/// `RESET` wipes the connection state server-side. The client must forget it too,
/// otherwise a later reconnection restores state the caller explicitly discarded.
#[tokio::test]
#[serial]
async fn reset_discards_the_state_the_reconnect_would_restore() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client.client_setname("reset_probe").await?;
    client.select(7).await?;
    client.reset().await?;

    force_reconnection(&client).await?;

    let name: Option<String> = client.client_getname().await?;
    assert_eq!(
        None, name,
        "a name discarded by RESET must not come back on reconnection"
    );

    let info: String = client.send(cmd("CLIENT").arg("INFO"), None).await?;
    let db = info
        .split(' ')
        .find_map(|field| field.strip_prefix("db="))
        .unwrap_or_default();
    assert_eq!(
        "0", db,
        "a database discarded by RESET must not come back on reconnection"
    );

    Ok(())
}

/// A caller that authenticates as another user at runtime keeps that identity
/// only until the socket dies: the handshake re-authenticates from the config,
/// so the connection silently comes back as whoever the config names.
#[tokio::test]
#[serial]
async fn runtime_authentication_is_restored_after_reconnect() -> Result<()> {
    let client = reconnecting_test_client().await?;

    client
        .acl_setuser(
            "auth_probe",
            ["on", ">auth_probe_password", "~*", "&*", "+@all"],
        )
        .await?;
    client
        .auth(Some("auth_probe"), "auth_probe_password")
        .await?;

    force_reconnection(&client).await?;

    let whoami: String = client.acl_whoami().await?;
    assert_eq!(
        "auth_probe", whoami,
        "the identity a caller authenticated as must survive a reconnection"
    );

    client.acl_deluser("auth_probe").await?;
    Ok(())
}
