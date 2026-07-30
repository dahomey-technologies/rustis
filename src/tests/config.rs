use crate::{
    ClientError, Error, Result,
    client::{Client, Config, IntoConfig, SentinelConfig, ServerConfig},
    commands::{ClientKillOptions, ConnectionCommands, FlushingMode, ServerCommands},
    tests::{get_default_host, get_default_port, get_test_client, log_try_init},
};
use serial_test::serial;

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

#[test]
fn display_masks_password() -> Result<()> {
    // Display is the natural way to log a config; it must never leak the
    // password in clear text.
    assert_eq!(
        "redis://:***@127.0.0.1",
        "redis://:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:***@127.0.0.1",
        "redis://username:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice?sentinel_username=foo&sentinel_password=***",
        "redis+sentinel://127.0.0.1:6379/myservice?sentinel_username=foo&sentinel_password=bar"
            .into_config()?
            .to_string()
    );

    // Debug must not leak the password either.
    let debug = format!("{:?}", "redis://username:pwd@127.0.0.1".into_config()?);
    assert!(!debug.contains("pwd"), "Debug leaked the password: {debug}");
    Ok(())
}

#[test]
fn into_config() -> Result<()> {
    assert_eq!("redis://127.0.0.1", "127.0.0.1".into_config()?.to_string());
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
        "redis://:***@127.0.0.1",
        "redis://:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:***@127.0.0.1",
        "redis://username:pwd@127.0.0.1".into_config()?.to_string()
    );
    assert_eq!(
        "redis://username:***@127.0.0.1/1",
        "redis://username:pwd@127.0.0.1/1"
            .into_config()?
            .to_string()
    );
    #[cfg(any(feature = "native-tls", feature = "rustls"))]
    assert_eq!(
        "rediss://username:***@127.0.0.1/1",
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
        "redis://127.0.0.1?keep_alive=60000",
        "redis://127.0.0.1?keep_alive=60000"
            .into_config()?
            .to_string()
    );
    // the default keep-alive is implicit in the URL
    assert_eq!(
        "redis://127.0.0.1",
        "redis://127.0.0.1?keep_alive=30000"
            .into_config()?
            .to_string()
    );
    // 0 means "no keep-alive" and must survive a round-trip
    assert_eq!(
        "redis://127.0.0.1?keep_alive=0",
        "redis://127.0.0.1?keep_alive=0".into_config()?.to_string()
    );
    assert_eq!(
        "redis://127.0.0.1?no_delay=false",
        "redis://127.0.0.1?no_delay=false"
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
        "redis+sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice/1",
        "redis-sentinel://127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice/1"
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
        "redis+sentinel://username:***@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice",
        "redis+sentinel://username:pwd@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://:***@127.0.0.1:6379,127.0.0.1:6380,127.0.0.1:6381/myservice",
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
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=100&sentinel_username=foo&sentinel_password=***",
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=100&sentinel_username=foo&sentinel_password=***"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice?sentinel_username=foo&sentinel_password=***",
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=250&sentinel_username=foo&sentinel_password=***"
            .into_config()?
            .to_string()
    );

    assert_eq!(
        "redis+sentinel://127.0.0.1:6379/myservice?connect_timeout=100&wait_between_failures=100&sentinel_username=foo&sentinel_password=***",
        "redis+sentinel://127.0.0.1:6379/myservice?connect_timeout=100&wait_between_failures=100&sentinel_username=foo&sentinel_password=***"
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

    Ok(())
}

#[test]
fn an_unknown_query_parameter_is_rejected() {
    // A misspelled knob used to be dropped without a word, leaving the default
    // in place while the caller believed they had set it.
    for uri in [
        "redis://127.0.0.1?param=value",
        "redis://127.0.0.1?commandtimeout=5000",
        "redis://127.0.0.1?reconnection=constant",
        "redis://127.0.0.1?command_timeout=5000&read_timeout=5000",
        "redis+sentinel://127.0.0.1:6379/myservice?sentinel_user=foo",
        "redis+cluster://127.0.0.1:6379?sentinel_username=foo",
    ] {
        let Err(Error::Client(ClientError::InvalidUri(message))) = uri.into_config() else {
            panic!("`{uri}` should be rejected as an unknown query parameter");
        };
        assert!(
            message.contains("unknown"),
            "`{uri}`: unhelpful message `{message}`"
        );
    }
}

#[test]
fn an_unparsable_query_parameter_value_is_rejected() {
    for uri in [
        "redis://127.0.0.1?command_timeout=5s",
        "redis://127.0.0.1?connect_timeout=5000ms",
        "redis://127.0.0.1?keep_alive=abc",
        "redis://127.0.0.1?no_delay=yes",
        "redis://127.0.0.1?auto_resubscribe=1",
        "redis://127.0.0.1?auto_remonitor=",
        "redis://127.0.0.1?retry_on_error=maybe",
        "redis://127.0.0.1?max_command_attempts=-1",
        "redis+sentinel://127.0.0.1:6379/myservice?wait_between_failures=250ms",
    ] {
        let Err(Error::Client(ClientError::InvalidUri(message))) = uri.into_config() else {
            panic!("`{uri}` should be rejected as an unparsable parameter value");
        };
        let name = uri.rsplit_once('?').unwrap().1.split('=').next().unwrap();
        assert!(
            message.contains(name),
            "`{uri}`: message `{message}` does not name the offending parameter"
        );
    }
}

#[tokio::test]
#[serial]
async fn connect_timeout() -> Result<()> {
    log_try_init();
    let client = Client::connect("redis://127.0.0.1:6379?connect_timeout=10000").await?;
    client.flushdb(FlushingMode::Sync).await?;

    Ok(())
}

#[test]
fn the_default_config_detects_a_half_open_connection() {
    // With neither a command timeout nor a TCP keep-alive, a socket silently
    // dropped by a NAT or a load balancer is reported by nothing and every
    // awaiting caller parks forever. The keep-alive is what breaks that tie.
    let config = Config::default();

    assert_eq!(Some(std::time::Duration::from_secs(30)), config.keep_alive);
}

#[test]
fn tuning_defaults_preserve_the_historical_hardcoded_values() {
    // These knobs were compile-time constants before they became configurable.
    // Their defaults are the values that shipped, so exposing them changes
    // nothing for a caller who does not touch them.
    let config = Config::default();

    assert_eq!(64 * 1024, config.buffers.read_capacity);
    assert_eq!(64 * 1024, config.buffers.tape_capacity);
    assert_eq!(8, config.buffers.shrink_factor);
    assert_eq!(16, config.buffers.shrink_hysteresis);

    assert_eq!(128, config.limits.max_nesting_depth);
    assert_eq!(512 * 1024 * 1024, config.limits.max_bulk_length);
    assert_eq!(128 * 1024 * 1024, config.limits.max_collection_length);

    assert_eq!(48, config.max_messages_per_wave);
    assert_eq!(10, SentinelConfig::default().max_discovery_rounds);
}

#[test]
fn a_default_config_validates() {
    assert!(Config::default().validate().is_ok());
}

#[test]
fn validate_rejects_knobs_whose_zero_value_would_break_the_connection() {
    // Every one of these is a divisor, a loop bound or a capacity whose zero
    // value does not degrade behaviour but removes it: no message is ever
    // flushed, no collection is ever accepted, no discovery round is ever run.
    fn assert_rejected(name: &str, zero_it: impl FnOnce(&mut Config)) {
        let mut config = Config::default();
        zero_it(&mut config);
        assert!(
            matches!(
                config.validate(),
                Err(Error::Client(ClientError::InvalidConfig(_)))
            ),
            "{name} = 0 must be rejected"
        );
    }

    assert_rejected("read_capacity", |c| c.buffers.read_capacity = 0);
    assert_rejected("tape_capacity", |c| c.buffers.tape_capacity = 0);
    assert_rejected("shrink_factor", |c| c.buffers.shrink_factor = 0);
    assert_rejected("shrink_hysteresis", |c| c.buffers.shrink_hysteresis = 0);
    assert_rejected("max_nesting_depth", |c| c.limits.max_nesting_depth = 0);
    assert_rejected("max_bulk_length", |c| c.limits.max_bulk_length = 0);
    assert_rejected("max_collection_length", |c| {
        c.limits.max_collection_length = 0
    });
    assert_rejected("max_messages_per_wave", |c| c.max_messages_per_wave = 0);
}

#[test]
fn validate_rejects_a_zero_sentinel_discovery_round_cap() {
    // Zero rounds means discovery gives up before contacting any Sentinel.
    let mut config = Config::default();
    let mut sentinel_config = SentinelConfig {
        instances: vec![("127.0.0.1".to_owned(), 26379)],
        service_name: "myservice".to_owned(),
        ..Default::default()
    };
    sentinel_config.max_discovery_rounds = 0;
    config.server = ServerConfig::Sentinel(sentinel_config);

    assert!(matches!(
        config.validate(),
        Err(Error::Client(ClientError::InvalidConfig(_)))
    ));
}

#[test]
fn validate_names_the_offending_knob() {
    // The error must say which knob is wrong: a config rejected at connect time
    // with an opaque message is the worst kind of startup failure.
    let mut config = Config::default();
    config.limits.max_bulk_length = 0;
    let Err(Error::Client(ClientError::InvalidConfig(message))) = config.validate() else {
        panic!("expected an InvalidConfig error");
    };
    assert!(
        message.contains("max_bulk_length"),
        "message did not name the knob: {message}"
    );
}
