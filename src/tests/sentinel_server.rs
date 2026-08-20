//! The `sentinel` tests that need a live Redis. The ones that need none stay in
//! `sentinel.rs`.

use crate::{
    Result,
    client::{Client, IntoConfig, ReconnectionConfig, ServerConfig},
    commands::{
        ConnectionCommands, GenericCommands, InternalCommands, ReplicaOfOptions, SentinelCommands,
        SentinelSimulateFailureMode, ServerCommands, StringCommands,
    },
    resp::cmd,
    tests::{
        SPARE_SENTINEL_SERVICE, get_default_host, get_sentinel_master_test_client,
        get_sentinel_master_test_uri, get_sentinel_test_client, get_spare_sentinel_test_client,
        log_try_init,
    },
};
use serial_test::serial;
use std::{collections::HashMap, time::Duration};

/// Connects once the spare Sentinel answers again. Connecting to a process that
/// is still starting succeeds and then breaks on the first command, so the ping
/// is the part that matters.
async fn wait_for_spare_sentinel_up() -> Result<Client> {
    for _ in 0..150 {
        if let Ok(client) = get_spare_sentinel_test_client().await
            && client.ping::<String>("").await.is_ok()
        {
            return Ok(client);
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    panic!("the spare Sentinel never came back");
}

async fn reset_spare_sentinel_topology(sentinel: &Client) -> Result<()> {
    let host = get_default_host();
    // The address the servers must dial to reach each other, which is not the
    // one this test connects through: inside a container `localhost` is that
    // container, so a REPLICAOF built from it points a server at itself. The
    // Sentinel knows the announced address, because it is the one it monitors.
    let announced_ip = sentinel.sentinel_master(SPARE_SENTINEL_SERVICE).await?.ip;

    // Whichever of the pair currently leads is taken as the master rather than
    // forced back to 6383: a failover swaps the roles for good, and a test that
    // insists on the original one is fighting the state it just created.
    let mut pair = Vec::new();
    for port in [6383u16, 6384] {
        let client = Client::connect(format!("{host}:{port}")).await?;
        let info: String = client.send(cmd("INFO").arg("replication"), None).await?;
        let is_master = info.lines().any(|line| line.trim() == "role:master");
        pair.push((port, client, is_master));
    }
    let master_port = pair
        .iter()
        .find(|(.., is_master)| *is_master)
        .map_or(6383, |(port, ..)| *port);

    for (port, client, _) in &pair {
        if *port == master_port {
            client.replicaof(ReplicaOfOptions::no_one()).await?;
        } else {
            client
                .replicaof(ReplicaOfOptions::master(&announced_ip, master_port))
                .await?;
        }
    }

    // REMOVE then MONITOR rather than RESET: a restarted Sentinel rebuilds its
    // config file from the compose command line, so it may be monitoring an
    // address that is no longer the master. RESET only re-reads that address,
    // where MONITOR replaces it.
    let _: Result<()> = sentinel.sentinel_remove(SPARE_SENTINEL_SERVICE).await;
    sentinel
        .sentinel_monitor(SPARE_SENTINEL_SERVICE, &announced_ip, master_port, 1)
        .await?;

    wait_for_synced_replica(sentinel).await
}

/// A failover needs a replica that has caught up. Right after one, the freshly
/// demoted master is still syncing, and a Sentinel asked to fail over again
/// answers `NOGOODSLAVE` instead of starting an election — so anything that
/// depends on an election happening has to wait for this first.
async fn wait_for_synced_replica(sentinel: &Client) -> Result<()> {
    wait_until("the replica catches up", || async {
        has_synced_replica(sentinel).await
    })
    .await
}

async fn has_synced_replica(sentinel: &Client) -> Result<bool> {
    let Ok(replicas) = sentinel.sentinel_replicas(SPARE_SENTINEL_SERVICE).await else {
        return Ok(false);
    };

    Ok(replicas.len() == 1 && replicas[0].master_link_status == "ok")
}

async fn sentinel_run_id(client: &Client) -> Result<String> {
    let info: String = client.send(cmd("INFO").arg("server"), None).await?;

    Ok(info
        .lines()
        .find_map(|line| line.strip_prefix("run_id:"))
        .expect("INFO server always reports a run_id")
        .trim()
        .to_owned())
}

/// A Sentinel converges on its own schedule — an election, a promotion and a
/// restart all take as long as they take.
async fn wait_until<F, Fut>(label: &str, mut condition: F) -> Result<()>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<bool>>,
{
    for _ in 0..150 {
        if condition().await? {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    panic!("{}: condition still false after 30s", label);
}

#[tokio::test]
#[serial]
async fn connection() -> Result<()> {
    let client = get_sentinel_master_test_client().await?;
    client.hello(Default::default()).await?;

    Ok(())
}

/// A sentinel connection redials through the sentinels to find the master, on a path
/// of its own. The connection state the caller set must be replayed there too — it is
/// the same socket loss as in the standalone case.
#[tokio::test]
#[serial]
async fn sentinel_connection_state_is_restored_after_reconnect() -> Result<()> {
    let mut config = get_sentinel_master_test_uri().into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = Client::connect(config).await?;
    let mut on_reconnect = client.on_reconnect();

    client.client_setname("sentinel_restore").await?;

    client.send_and_forget(cmd("PING").kill_connection_on_read(1), None)?;
    on_reconnect
        .recv()
        .await
        .expect("the client should have reconnected");

    let name: Option<String> = client.client_getname().await?;
    assert_eq!(
        Some("sentinel_restore".to_owned()),
        name,
        "a name set at runtime must survive a reconnection through the sentinels"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn connection_with_failures() -> Result<()> {
    log_try_init();
    let client =
        Client::connect("redis+sentinel://127.0.0.1:1234,127.0.0.1:26379/myservice").await?;
    client.hello(Default::default()).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn config_get_set() -> Result<()> {
    // connect to the sentinel instance directly for these commands
    let client = get_sentinel_test_client().await?;

    client.sentinel_config_set("sentinel-user", "user").await?;
    client.sentinel_config_set("sentinel-pass", "pwd").await?;

    let configs: HashMap<String, String> = client.sentinel_config_get("sentinel-*").await?;
    assert_eq!(2, configs.len());
    assert_eq!(Some(&"user".to_owned()), configs.get("sentinel-user"));
    assert_eq!(Some(&"pwd".to_owned()), configs.get("sentinel-pass"));

    client.sentinel_config_set("sentinel-user", "").await?;
    client.sentinel_config_set("sentinel-pass", "").await?;

    let configs: HashMap<String, String> = client.sentinel_config_get("toto").await?;
    assert_eq!(0, configs.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_ckquorum() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    // The whole point of CKQUORUM is the status line it answers; an error only
    // tells the caller the quorum is unreachable, never how close it was.
    let status: String = client.sentinel_ckquorum("myservice").await?;
    assert!(
        status.starts_with("OK") && status.contains("usable Sentinels"),
        "unexpected CKQUORUM status: {status}"
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_flushconfig() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    client.sentinel_flushconfig().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_info_cache() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    let result: HashMap<String, Vec<(u64, String)>> =
        client.sentinel_info_cache("myservice").await?;
    assert_eq!(1, result.len());
    assert!(result.contains_key("myservice"));
    assert!(result.get("myservice").unwrap().len() == 2); // 1 master & 1 replica

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_master() {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await.unwrap();

    let result = client.sentinel_master("myservice").await.unwrap();
    assert_eq!("master", result.flags);
    //assert_eq!(2, result.num_other_sentinels);
    assert_eq!(2, result.quorum);
}

#[tokio::test]
#[serial]
async fn sentinel_masters() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    let result = client.sentinel_masters().await?;
    assert_eq!(1, result.len());
    assert_eq!("master", result[0].flags);
    //assert_eq!(2, result[0].num_other_sentinels);
    assert_eq!(2, result[0].quorum);

    Ok(())
}

// #[tokio::test]
// #[serial]
// async fn sentinel_remove_and_monitor() -> Result<()> {
//     // connect to the sentinel instance directly for these commands
//     let client = get_sentinel_test_client().await?;

//     let master_info = client.sentinel_master("myservice").await?;

//     client.sentinel_remove("myservice").await?;
//     client
//         .sentinel_monitor(
//             "myservice",
//             master_info.ip,
//             master_info.port,
//             master_info.quorum,
//         )
//         .await?;

//     client.sentinel_reset("myservice").await?;

//     Ok(())
// }

#[tokio::test]
#[serial]
async fn sentinel_set() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    client
        .sentinel_set(
            "myservice",
            [
                ("down-after-milliseconds", 1000),
                ("failover-timeout", 1000),
            ],
        )
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_myid() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    let id = client.sentinel_myid().await?;
    assert!(!id.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_pending_scripts() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let sentinel_client = get_sentinel_test_client().await?;

    let result = sentinel_client.sentinel_pending_scripts().await?;
    assert!(result.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_replicas() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let sentinel_client = get_sentinel_test_client().await?;

    let result = sentinel_client.sentinel_replicas("myservice").await?;
    assert_eq!(1, result.len());
    assert_eq!("slave", result[0].flags);
    assert_eq!(6382, result[0].port);
    assert_eq!(6381, result[0].master_port);

    Ok(())
}

#[tokio::test]
#[serial]
async fn sentinel_sentinels() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    let result = client.sentinel_sentinels("myservice").await?;
    assert!(!result.is_empty());
    assert!(result[0].flags.contains("sentinel"));
    //assert_eq!(26379, result[0].port);

    Ok(())
}

// #[tokio::test]
// #[serial]
// async fn sentinel_reset() -> Result<()> {
//     // connect to the sentinel instance directly for this command
//     let client = get_sentinel_test_client().await?;

//     let num = client.sentinel_reset("myservice").await?;
//     assert_eq!(1, num);

//     Ok(())
// }

#[tokio::test]
#[serial]
async fn sentinel_get_master_addr_by_name() -> Result<()> {
    // connect to the sentinel instance directly for this command
    let client = get_sentinel_test_client().await?;

    let addr = client.sentinel_get_master_addr_by_name("myservice").await?;
    let Some((ip, port)) = addr else {
        panic!("sentinel does not know the master of a service it monitors");
    };
    assert!(!ip.is_empty());
    assert_eq!(6381, port);

    // An unmonitored service has no address, which is the whole point of the
    // `Option`: the server answers a null array rather than an error.
    let addr = client.sentinel_get_master_addr_by_name("unknown").await?;
    assert!(addr.is_none());

    Ok(())
}

/// SENTINEL FAILOVER sent for real, against a deployment nothing else
/// monitors. The wire-form test above cannot check the declared response type:
/// `R` is only ever wrong against a reply, and this command had never had one.
#[tokio::test]
#[serial]
async fn sentinel_failover() -> Result<()> {
    let client = wait_for_spare_sentinel_up().await?;
    reset_spare_sentinel_topology(&client).await?;

    // Which of the pair leads depends on what an earlier run left behind, so
    // the assertion is that the address moved, not where it moved to.
    let before = client
        .sentinel_get_master_addr_by_name(SPARE_SENTINEL_SERVICE)
        .await?
        .expect("the spare Sentinel monitors spareservice");

    client.sentinel_failover(SPARE_SENTINEL_SERVICE).await?;
    wait_until("failover moves the master address", || async {
        let addr = client
            .sentinel_get_master_addr_by_name(SPARE_SENTINEL_SERVICE)
            .await
            .unwrap_or(None);
        Ok(addr.is_some_and(|addr| addr != before))
    })
    .await?;

    Ok(())
}

/// SIMULATE-FAILURE arms a crash rather than causing one, so its reply proves
/// only that the flag was accepted; the failover after it is what makes the
/// effect observable — and consuming the flag here is also what leaves the
/// Sentinel unarmed for whatever runs next.
///
/// Deliberately not chained onto the test above: a Sentinel that has just
/// failed over needs an unpredictable while before it can elect again, and a
/// second act reusing the first one's aftermath is a test measuring that delay
/// rather than the command. Both tests instead rebuild the topology they need.
#[tokio::test]
#[serial]
async fn sentinel_simulate_failure() -> Result<()> {
    let client = wait_for_spare_sentinel_up().await?;
    reset_spare_sentinel_topology(&client).await?;

    let run_id_before = sentinel_run_id(&client).await?;
    client
        .sentinel_simulate_failure(SentinelSimulateFailureMode::CrashAfterElection)
        .await?;
    // The failover is asked for inside the loop rather than once before it: a
    // Sentinel that failed over recently answers `NOGOODSLAVE` or simply
    // declines to elect for a while, and there is no state to poll that says
    // when it will accept. Retrying until the crash is observed removes the
    // guess, and asking twice costs nothing — the flag only fires once.
    //
    // A restarted Sentinel announces a new run id, which is the difference
    // between "it crashed and came back" and "it never crashed".
    wait_until("the simulated crash restarts the Sentinel", || async {
        let Ok(client) = get_spare_sentinel_test_client().await else {
            return Ok(false);
        };

        // The reply never comes when the flag does fire: the Sentinel dies
        // mid-election, by design.
        let _: Result<()> = client.sentinel_failover(SPARE_SENTINEL_SERVICE).await;

        Ok(sentinel_run_id(&client)
            .await
            .is_ok_and(|run_id| run_id != run_id_before))
    })
    .await?;

    Ok(())
}

/// A master demoted to replica keeps serving the connections it already had:
/// `REPLICAOF` closes none of them. So nothing in the transport tells a client its
/// node changed role — the only thing that does is the `READONLY` the demoted node
/// answers to the next write, and that has to send the client back through the
/// sentinels. Otherwise it writes to a replica for as long as the socket holds,
/// which is forever when no timeout is configured.
///
/// The roles are swapped by hand rather than through `SENTINEL FAILOVER`: a
/// Sentinel-driven failover also drops the demoted node's client connections, so a
/// client would recover from the socket loss and the test would pass whatever the
/// `READONLY` handling does.
#[tokio::test]
#[serial]
async fn a_write_recovers_after_the_master_is_demoted() -> Result<()> {
    log_try_init();
    let sentinel = wait_for_spare_sentinel_up().await?;
    reset_spare_sentinel_topology(&sentinel).await?;

    let host = get_default_host();
    let mut config =
        format!("redis+sentinel://{host}:26382/{SPARE_SENTINEL_SERVICE}").into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = Client::connect(config).await?;

    // A write before the demotion: it proves the client reached a real master, so a
    // failure afterwards is the demotion and not a deployment that was never ready.
    client.set("sentinel_demoted_master", "before").await?;

    // The address the servers dial to reach each other, which is not the one this
    // test connects through — see `reset_spare_sentinel_topology`.
    let master = sentinel.sentinel_master(SPARE_SENTINEL_SERVICE).await?;
    let announced_ip = master.ip;
    let demoted_port = master.port;
    let promoted_port = if demoted_port == 6383 { 6384 } else { 6383 };

    // Promote first, so the node being demoted has a real master to follow.
    let promoted = Client::connect(format!("{host}:{promoted_port}")).await?;
    promoted.replicaof(ReplicaOfOptions::no_one()).await?;
    let demoted = Client::connect(format!("{host}:{demoted_port}")).await?;
    demoted
        .replicaof(ReplicaOfOptions::master(&announced_ip, promoted_port))
        .await?;

    // Point the Sentinel at the new master rather than waiting for it to notice: an
    // election is its own subject, and the client under test only ever asks the
    // Sentinel where the master is.
    let _: Result<()> = sentinel.sentinel_remove(SPARE_SENTINEL_SERVICE).await;
    sentinel
        .sentinel_monitor(SPARE_SENTINEL_SERVICE, &announced_ip, promoted_port, 1)
        .await?;

    // The first write after the demotion is expected to fail — with the `READONLY`
    // the demoted node answers, which its caller is entitled to see. What must not
    // happen is that it keeps failing: a write only ever succeeds on a master, so a
    // success here is the proof the client went back through the sentinels.
    wait_until("the client writes to the promoted master again", || async {
        Ok(client.set("sentinel_demoted_master", "after").await.is_ok())
    })
    .await?;

    let value: String = client.get("sentinel_demoted_master").await?;
    assert_eq!("after", value);

    client.del("sentinel_demoted_master").await?;
    Ok(())
}

/// The `READONLY` recovery costs the caller a refused write: the client only
/// learns the master moved by being told off for writing to it. Polling the
/// Sentinels closes that gap — the connection is remade while nothing is being
/// sent, so the next write is the first one and it succeeds.
///
/// The roles are swapped exactly as in `a_write_recovers_after_the_master_is_demoted`,
/// which is what makes this test falsifiable: `REPLICAOF` drops no connection and
/// `SENTINEL MONITOR` publishes no `+switch-master`, so neither the socket nor the
/// subscription can be what notices. With no poll the reconnection never comes and
/// the wait below times out.
#[tokio::test]
#[serial]
async fn the_master_is_rediscovered_before_a_write_is_refused() -> Result<()> {
    log_try_init();
    let sentinel = wait_for_spare_sentinel_up().await?;
    reset_spare_sentinel_topology(&sentinel).await?;

    let host = get_default_host();
    let mut config =
        format!("redis+sentinel://{host}:26382/{SPARE_SENTINEL_SERVICE}").into_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    if let ServerConfig::Sentinel(sentinel_config) = &mut config.server {
        // Short enough that the test does not wait on the 10-second default.
        sentinel_config.master_check_interval = Some(Duration::from_millis(200));
    }
    let client = Client::connect(config).await?;

    client.set("sentinel_polled_master", "before").await?;
    let reconnections_before = client.stats().reconnections;

    let master = sentinel.sentinel_master(SPARE_SENTINEL_SERVICE).await?;
    let announced_ip = master.ip;
    let demoted_port = master.port;
    let promoted_port = if demoted_port == 6383 { 6384 } else { 6383 };

    let promoted = Client::connect(format!("{host}:{promoted_port}")).await?;
    promoted.replicaof(ReplicaOfOptions::no_one()).await?;
    let demoted = Client::connect(format!("{host}:{demoted_port}")).await?;
    demoted
        .replicaof(ReplicaOfOptions::master(&announced_ip, promoted_port))
        .await?;

    let _: Result<()> = sentinel.sentinel_remove(SPARE_SENTINEL_SERVICE).await;
    sentinel
        .sentinel_monitor(SPARE_SENTINEL_SERVICE, &announced_ip, promoted_port, 1)
        .await?;

    // The reconnection is what is waited on, not a write: retrying a write until
    // it works is exactly the behaviour this test exists to distinguish from.
    wait_until(
        "the poll sends the client back through the sentinels",
        || {
            let client = client.clone();
            async move { Ok(client.stats().reconnections > reconnections_before) }
        },
    )
    .await?;

    // No write has been attempted since the demotion, so this is the first one.
    client.set("sentinel_polled_master", "after").await?;

    let value: String = client.get("sentinel_polled_master").await?;
    assert_eq!("after", value);

    client.del("sentinel_polled_master").await?;
    Ok(())
}

/// The client spec requires a client to learn the Sentinel fleet from the
/// Sentinel it reached, so a configuration naming one instance survives that
/// instance being replaced. Without it the list is frozen at whatever the URI
/// said, and a redeployed fleet leaves the client with no reachable Sentinel.
#[tokio::test]
#[serial]
async fn a_sentinel_connection_learns_the_other_instances() -> Result<()> {
    use crate::{
        ConnectionState,
        client::{Config, SentinelConfig},
        network::SentinelConnection,
    };

    log_try_init();

    // One instance only: the other two must be discovered.
    let sentinel_config = SentinelConfig {
        instances: vec![(get_default_host(), 26379)],
        service_name: "myservice".to_owned(),
        ..Default::default()
    };
    let mut connection_state = ConnectionState::default();
    let connection =
        SentinelConnection::connect(&sentinel_config, &Config::default(), &mut connection_state)
            .await?;

    let known = connection.known_instances();
    let ports: Vec<u16> = known.iter().map(|(_, port)| *port).collect();
    assert!(
        ports.contains(&26380) && ports.contains(&26381),
        "the fleet must be learned from the answering Sentinel, got {known:?}"
    );
    assert_eq!(
        Some(26379),
        ports.first().copied(),
        "the Sentinel that answered must stay first, so the next discovery starts there"
    );

    Ok(())
}
