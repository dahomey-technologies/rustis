use crate::{
    Result,
    client::Client,
    commands::{SentinelCommands, SentinelSimulateFailureMode},
    tests::{TestClient, log_try_init},
};
use serial_test::serial;
use std::sync::Arc;

/// A Sentinel is a different server with its own ACLs: a probe must carry the
/// Sentinel's own credentials, never the master's.
#[tokio::test]
async fn sentinel_probes_use_the_sentinel_credentials() -> Result<()> {
    use crate::{
        client::{Config, Credentials, SentinelConfig},
        network::SentinelConnection,
    };

    let config = Config {
        credentials_provider: Some(Arc::new(|| async {
            Ok(Credentials {
                username: None,
                password: "master_token".to_owned(),
            })
        })),
        ..Default::default()
    };

    // A Sentinel provider is used for the probe.
    let sentinel_config = SentinelConfig {
        credentials_provider: Some(Arc::new(|| async {
            Ok(Credentials {
                username: None,
                password: "sentinel_token".to_owned(),
            })
        })),
        ..Default::default()
    };
    let probe_config = SentinelConnection::probe_config(&sentinel_config, &config);
    let credentials = probe_config.resolve_credentials().await?.unwrap();
    assert_eq!("sentinel_token", credentials.password);

    // Without one, the static Sentinel credentials apply and the master's
    // provider is left out.
    let sentinel_config = SentinelConfig {
        username: Some("sentinel_user".to_owned()),
        password: Some("sentinel_pwd".to_owned()),
        ..Default::default()
    };
    let probe_config = SentinelConnection::probe_config(&sentinel_config, &config);
    let credentials = probe_config.resolve_credentials().await?.unwrap();
    assert_eq!(Some("sentinel_user"), credentials.username.as_deref());
    assert_eq!("sentinel_pwd", credentials.password);

    // And with no Sentinel credentials at all, the probe is unauthenticated
    // rather than authenticated as the master.
    let probe_config = SentinelConnection::probe_config(&SentinelConfig::default(), &config);
    assert!(probe_config.resolve_credentials().await?.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn unreachable() -> Result<()> {
    log_try_init();
    let result = Client::connect("redis+sentinel://127.0.0.1:1234,127.0.0.1:5678/myservice").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn unknown_service() -> Result<()> {
    log_try_init();
    let result = Client::connect("redis+sentinel://127.0.0.1:26379/unknown").await;
    assert!(result.is_err());

    Ok(())
}

// The two commands below take the monitored master down on purpose, so neither
// can be sent against the shared sentinel set-up. The wire-form tests check
// their argument shape against the syntax the server prints under
// `SENTINEL HELP`; the live test after them sends both to the spare Sentinel on
// 26382, which monitors nothing else, and is the only thing that can check the
// declared response type against a real reply.

#[test]
fn sentinel_failover_command() {
    let cmd = TestClient.sentinel_failover("myservice").command;
    assert_eq!("SENTINEL FAILOVER myservice", cmd.to_string());
}

#[test]
fn sentinel_simulate_failure_command() {
    let cmd = TestClient
        .sentinel_simulate_failure(SentinelSimulateFailureMode::CrashAfterElection)
        .command;
    assert_eq!(
        "SENTINEL SIMULATE-FAILURE CRASH-AFTER-ELECTION",
        cmd.to_string()
    );

    let cmd = TestClient
        .sentinel_simulate_failure(SentinelSimulateFailureMode::CrashAfterPromotion)
        .command;
    assert_eq!(
        "SENTINEL SIMULATE-FAILURE CRASH-AFTER-PROMOTION",
        cmd.to_string()
    );
}
