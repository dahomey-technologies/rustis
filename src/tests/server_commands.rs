use crate::{
    ClientError, ErrorKind, RedisError, RedisErrorKind, Result,
    client::{Client, ReconnectionConfig},
    commands::{
        AclCatOptions, AclDryRunOptions, AclGenPassOptions, AclLogOptions, BgsaveOptions,
        BlockingCommands, ClientInfo, ClientKillOptions, CommandDoc, CommandHistogram,
        CommandListOptions, ConnectionCommands, DebugCommands, FailOverOptions, FlushingMode,
        HotKeysInfo, HotKeysMetric, HotKeysStartOptions, InfoSection, LatencyHistoryEvent,
        LolWutOptions, MemoryUsageOptions, ModuleInfo, ModuleLoadexOptions, ReplicaOfOptions,
        RoleResult, ServerCommands, ShutdownOptions, SlowLogGetOptions, StringCommands,
    },
    resp::Value,
    spawn,
    tests::{
        TestClient, get_default_config, get_exclusive_test_client,
        get_exclusive_test_client_with_config, get_sentinel_test_client, get_test_client,
    },
};
use futures_util::StreamExt;
use serial_test::serial;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

#[tokio::test]
#[serial]
async fn acl_cat() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let categories: Vec<String> = client.acl_cat(AclCatOptions::default()).await?;
    assert!(categories.contains(&"dangerous".to_owned()));

    let dangerous_commands: HashSet<String> = client
        .acl_cat(AclCatOptions::category_name("dangerous"))
        .await?;
    assert!(dangerous_commands.contains("flushdb"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_deluser() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.acl_setuser("foo", Vec::<String>::new()).await?;
    client.acl_setuser("bar", Vec::<String>::new()).await?;
    let deleted = client.acl_deluser(["foo", "bar"]).await?;
    assert_eq!(2, deleted);

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_dryrun() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.acl_setuser("VIRGINIA", ["+SET", "~*"]).await?;
    let result: String = client
        .acl_dryrun(
            "VIRGINIA",
            "SET",
            AclDryRunOptions::default().arg("foo").arg("bar"),
        )
        .await?;
    assert_eq!("OK", result);

    let result: String = client
        .acl_dryrun("VIRGINIA", "GET", AclDryRunOptions::default().arg("foo"))
        .await?;
    assert_eq!(
        "User VIRGINIA has no permissions to run the 'get' command",
        result
    );

    client.acl_deluser("VIRGINIA").await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_genpass() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let password: String = client.acl_genpass(AclGenPassOptions::default()).await?;
    assert_eq!(64, password.len());

    let password: String = client.acl_genpass(AclGenPassOptions::bits(32)).await?;
    assert_eq!(8, password.len());

    let password: String = client.acl_genpass(AclGenPassOptions::bits(5)).await?;
    assert_eq!(2, password.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_getuser() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.acl_setuser("foo", Vec::<String>::new()).await?;
    let rules: HashMap<String, Value> = client.acl_getuser("foo").await?;
    tracing::debug!("rules: {rules:?}");
    // default `commands` rule
    assert!(matches!(rules.get("commands"), Some(Value::BulkString(rule)) if rule == b"-@all"));

    client.acl_deluser("foo").await?;

    Ok(())
}

#[tokio::test]
async fn acl_help() -> Result<()> {
    let client = get_test_client().await?;

    let result: Vec<String> = client.acl_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_list() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let users: Vec<String> = client.acl_list().await?;
    assert_eq!(1, users.len());
    assert!(users[0].starts_with("user default"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_load() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.acl_load().await;
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
async fn acl_log() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.auth(Some("someuser"), "wrongpassword").await;
    assert!(result.is_err());

    let logs: Vec<HashMap<String, Value>> = client.acl_log(AclLogOptions::count(1)).await?;
    assert_eq!(1, logs.len());
    assert!(matches!(logs[0].get("reason"), Some(Value::BulkString(reason)) if reason == b"auth"));
    let client_info: String = logs[0].get("client-info").unwrap().to_string();
    let client_info = ClientInfo::from_line(&client_info)?;
    assert_eq!("auth", client_info.cmd);

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_save() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.acl_save().await;
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
async fn acl_setuser() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // activate user + password + remove all key patterns + allow all commands
    client
        .acl_setuser("foo", ["on", ">pwd", "resetkeys", "allcommands"])
        .await?;

    client.auth(Some("foo"), "pwd").await?;

    let result = client.set("key", "value").await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::NoPerm,
            description: _
        })
    ));

    client.acl_setuser("foo", ["~key"]).await?;
    let _rules: HashMap<String, Value> = client.acl_getuser("foo").await?;
    client.set("key", "value").await?;

    client.close().await?;

    // new connection with default user because
    // Redis close the connection when deleting the current user.
    let client = get_test_client().await?;
    client.acl_deluser("foo").await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_users() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.acl_setuser("foo", Vec::<String>::new()).await?;
    client.acl_setuser("bar", Vec::<String>::new()).await?;

    let users: Vec<String> = client.acl_users().await?;
    assert_eq!(3, users.len());
    assert_eq!("bar", users[0]);
    assert_eq!("default", users[1]);
    assert_eq!("foo", users[2]);

    client.acl_deluser("foo").await?;
    client.acl_deluser("bar").await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn acl_whoami() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let current_user: String = client.acl_whoami().await?;
    assert_eq!("default", current_user);

    client
        .acl_setuser("foo", ["on", ">pwd", "+ACL|WHOAMI"])
        .await?;
    client.auth(Some("foo"), "pwd").await?;
    let current_user: String = client.acl_whoami().await?;
    assert_eq!("foo", current_user);

    client.auth(Some("default"), "").await?;
    client.acl_deluser("foo").await?;

    let current_user: String = client.acl_whoami().await?;
    assert_eq!("default", current_user);

    Ok(())
}

#[tokio::test]
#[serial]
async fn bgrewriteaof() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.bgrewriteaof().await?;
    assert!(result.starts_with("Background append only file rewriting "));

    Ok(())
}

#[tokio::test]
#[serial]
async fn bgsave() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.bgsave(BgsaveOptions::default().schedule()).await?;
    assert!(result.starts_with("Background saving "));

    Ok(())
}

#[tokio::test]
#[serial]
async fn command() -> Result<()> {
    let client = get_test_client().await?;

    let _command_infos = client.command().await?;

    Ok(())
}

#[tokio::test]
async fn command_help() -> Result<()> {
    let client = get_test_client().await?;

    let result: Vec<String> = client.command_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn command_info() -> Result<()> {
    let client = get_test_client().await?;

    let _command_infos = client.command_info("SORT").await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn command_count() -> Result<()> {
    let client = get_test_client().await?;

    let command_infos = client.command().await?;
    let num_commands = client.command_count().await?;
    assert_eq!(command_infos.len(), num_commands);

    Ok(())
}

#[tokio::test]
#[serial]
async fn command_docs() -> Result<()> {
    let client = get_test_client().await?;

    let _command_docs: HashMap<String, CommandDoc> =
        client.command_docs(["XADD", "GET", "SET"]).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn command_getkeys() -> Result<()> {
    let client = get_test_client().await?;

    let keys: Vec<String> = client
        .command_getkeys(["MSET", "a", "b", "c", "d", "e", "f"])
        .await?;
    assert!(keys.contains(&"a".to_owned()));
    assert!(keys.contains(&"c".to_owned()));
    assert!(keys.contains(&"e".to_owned()));

    let keys: Vec<String> = client
        .command_getkeys(["EVAL", "not consulted", "3", "key1", "key2", "key3", "arg1"])
        .await?;
    assert!(keys.contains(&"key1".to_owned()));
    assert!(keys.contains(&"key2".to_owned()));
    assert!(keys.contains(&"key3".to_owned()));

    let keys: Vec<String> = client
        .command_getkeys(["SORT", "mylist", "ALPHA", "STORE", "outlist"])
        .await?;
    assert!(keys.contains(&"mylist".to_owned()));
    assert!(keys.contains(&"outlist".to_owned()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn command_getkeysandflags() -> Result<()> {
    let client = get_test_client().await?;

    let keys_and_flags: HashMap<String, Vec<String>> = client
        .command_getkeysandflags(["MSET", "a", "b", "c", "d", "e", "f"])
        .await?;
    assert!(keys_and_flags.contains_key("a"));
    assert!(keys_and_flags.contains_key("c"));
    assert!(keys_and_flags.contains_key("e"));

    let keys_and_flags: HashMap<String, Vec<String>> = client
        .command_getkeysandflags(["EVAL", "not consulted", "3", "key1", "key2", "key3", "arg1"])
        .await?;
    assert!(keys_and_flags.contains_key("key1"));
    assert!(keys_and_flags.contains_key("key2"));
    assert!(keys_and_flags.contains_key("key3"));

    let keys_and_flags: HashMap<String, Vec<String>> = client
        .command_getkeysandflags(["LMOVE", "mylist1", "mylist2", "left", "left"])
        .await?;
    let flags = keys_and_flags.get("mylist1").unwrap();
    assert_eq!("RW", flags[0]);
    assert_eq!("access", flags[1]);
    assert_eq!("delete", flags[2]);
    let flags = keys_and_flags.get("mylist2").unwrap();
    assert_eq!("RW", flags[0]);
    assert_eq!("insert", flags[1]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn command_list() -> Result<()> {
    let client = get_test_client().await?;

    let all_commands: Vec<String> = client.command_list(CommandListOptions::default()).await?;
    assert!(!all_commands.is_empty());

    let string_commands: Vec<String> = client
        .command_list(CommandListOptions::default().filter_by_acl_category("string"))
        .await?;
    assert!(!string_commands.is_empty());
    assert!(string_commands.contains(&"get".to_owned()));
    assert!(string_commands.contains(&"set".to_owned()));

    let config_commands: Vec<String> = client
        .command_list(CommandListOptions::default().filter_by_pattern("config*"))
        .await?;
    assert!(!config_commands.is_empty());
    assert!(config_commands.contains(&"config|get".to_owned()));
    assert!(config_commands.contains(&"config|set".to_owned()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn config_get() -> Result<()> {
    let client = get_test_client().await?;

    let configs: HashMap<String, String> = client
        .config_get(["hash-max-listpack-entries", "zset-max-listpack-entries"])
        .await?;
    assert_eq!(2, configs.len());
    assert_eq!(
        Some(&"512".to_owned()),
        configs.get("hash-max-listpack-entries")
    );
    assert_eq!(
        Some(&"128".to_owned()),
        configs.get("zset-max-listpack-entries")
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn config_help() -> Result<()> {
    let client = get_test_client().await?;
    let result: Vec<String> = client.config_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn config_resetstat() -> Result<()> {
    let client = get_test_client().await?;

    client.config_resetstat().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn config_rewrite() -> Result<()> {
    let client = get_test_client().await?;

    let _result = client.config_rewrite().await;

    Ok(())
}

#[tokio::test]
#[serial]
async fn config_set() -> Result<()> {
    let client = get_test_client().await?;

    client
        .config_set([
            ("hash-max-listpack-entries", 513),
            ("zset-max-listpack-entries", 129),
        ])
        .await?;

    let configs: HashMap<String, String> = client
        .config_get(["hash-max-listpack-entries", "zset-max-listpack-entries"])
        .await?;
    assert_eq!(2, configs.len());
    assert_eq!(
        Some(&"513".to_owned()),
        configs.get("hash-max-listpack-entries")
    );
    assert_eq!(
        Some(&"129".to_owned()),
        configs.get("zset-max-listpack-entries")
    );

    client
        .config_set([
            ("hash-max-listpack-entries", 512),
            ("zset-max-listpack-entries", 128),
        ])
        .await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn dbsize() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .mset([("key1", "value1"), ("key2", "value2")])
        .await?;

    let size = client.dbsize().await?;
    assert_eq!(2, size);

    Ok(())
}

#[tokio::test]
#[serial]
async fn failover() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client.failover(FailOverOptions::default()).await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        }) if description == "FAILOVER requires connected replicas."
    ));

    Ok(())
}

#[tokio::test]
#[serial]
async fn flushdb() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    client0.set("key1", "value1").await?;
    client0.set("key2", "value2").await?;

    client1.set("key1", "value1").await?;
    client1.set("key2", "value2").await?;

    client0.flushdb(None).await?;

    let value: Value = client0.get("key1").await?;
    assert!(matches!(value, Value::Null));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::Null));

    let value: String = client1.get("key1").await?;
    assert_eq!("value1", value);

    let value: String = client1.get("key2").await?;
    assert_eq!("value2", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn flushall() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    client0.set("key1", "value1").await?;
    client0.set("key2", "value2").await?;

    client1.set("key1", "value1").await?;
    client1.set("key2", "value2").await?;

    client0.flushall(None).await?;

    let value: Value = client0.get("key1").await?;
    assert!(matches!(value, Value::Null));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::Null));

    let value: Value = client1.get("key1").await?;
    assert!(matches!(value, Value::Null));

    let value: Value = client1.get("key2").await?;
    assert!(matches!(value, Value::Null));

    Ok(())
}

#[tokio::test]
#[serial]
async fn info() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let info: String = client.info(Vec::<InfoSection>::new()).await?;
    assert!(!info.is_empty());

    let info: String = client
        .info([InfoSection::Cpu, InfoSection::Clients])
        .await?;
    assert!(info.contains("# CPU"));
    assert!(info.contains("# Clients"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn hotkeys() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // Leave no session behind from another run: STOP replies nil when nothing
    // is being tracked, and RESET is refused while a session is active.
    client.hotkeys_stop().await?;
    client.hotkeys_reset().await?;

    // Without a tracking session there is nothing to report at all.
    let results: Option<Vec<HotKeysInfo>> = client.hotkeys_get().await?;
    assert!(results.is_none());

    client
        .hotkeys_start(
            [HotKeysMetric::Cpu, HotKeysMetric::Net],
            HotKeysStartOptions::default()
                .count(5)
                .duration(60)
                .sample(1),
        )
        .await?;

    client.set("hot", "value").await?;
    for _ in 0..20 {
        let _: String = client.get("hot").await?;
    }

    // One entry per node; a standalone server reports a single one.
    let results: Vec<HotKeysInfo> = client.hotkeys_get().await?;
    assert_eq!(1, results.len());
    let info = &results[0];
    assert!(info.tracking_active);
    assert_eq!(1, info.sample_ratio);
    assert_eq!(vec![(0, 16383)], info.selected_slots);
    assert!(info.collection_start_time_unix_ms > 0);

    // Both metrics were requested, so both breakdowns are reported.
    let by_cpu_time_us = info.by_cpu_time_us.as_ref().unwrap();
    assert!(by_cpu_time_us.iter().any(|(key, _)| key == "hot"));
    let by_net_bytes = info.by_net_bytes.as_ref().unwrap();
    assert!(by_net_bytes.iter().any(|(key, _)| key == "hot"));
    assert!(info.total_cpu_time_user_ms.is_some());
    assert!(info.total_net_bytes.is_some());

    // Stopping preserves the results, only the flag flips.
    client.hotkeys_stop().await?;
    let results: Vec<HotKeysInfo> = client.hotkeys_get().await?;
    assert!(!results[0].tracking_active);
    assert!(
        results[0]
            .by_cpu_time_us
            .as_ref()
            .unwrap()
            .iter()
            .any(|(key, _)| key == "hot")
    );

    // A single metric leaves the other breakdown out of the reply entirely.
    client.hotkeys_reset().await?;
    client
        .hotkeys_start(
            [HotKeysMetric::Cpu],
            HotKeysStartOptions::default().duration(60),
        )
        .await?;
    let _: String = client.get("hot").await?;
    let results: Vec<HotKeysInfo> = client.hotkeys_get().await?;
    assert!(results[0].by_cpu_time_us.is_some());
    assert!(results[0].by_net_bytes.is_none());
    assert!(results[0].total_net_bytes.is_none());

    client.hotkeys_stop().await?;
    client.hotkeys_reset().await?;
    let results: Option<Vec<HotKeysInfo>> = client.hotkeys_get().await?;
    assert!(results.is_none());

    let help: Vec<String> = client.hotkeys_help().await?;
    assert!(!help.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn lastsave() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let lastsave = client.lastsave().await?;
    assert!(lastsave > 0);

    Ok(())
}

#[tokio::test]
#[serial]
async fn latency_doctor() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report: String = client.latency_doctor().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn latency_graph() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client.debug_sleep(Duration::from_millis(100)).await?;
    client.debug_sleep(Duration::from_millis(200)).await?;
    client.debug_sleep(Duration::from_millis(200)).await?;

    let report: String = client.latency_graph(LatencyHistoryEvent::Command).await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
async fn latency_help() -> Result<()> {
    let client = get_test_client().await?;

    let result: Vec<String> = client.latency_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn latency_histogram() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;
    client.set("key", "value").await?;

    let report: HashMap<String, CommandHistogram> = client.latency_histogram("set").await?;
    assert_eq!(1, report.len());
    assert!(report.get("set").unwrap().calls >= 10);

    Ok(())
}

#[tokio::test]
#[serial]
async fn latency_history() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client.debug_sleep(Duration::from_millis(100)).await?;
    client.debug_sleep(Duration::from_millis(200)).await?;
    client.debug_sleep(Duration::from_millis(200)).await?;

    let report: Vec<(u32, u32)> = client.latency_history(LatencyHistoryEvent::Command).await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn latency_latest() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client.debug_sleep(Duration::from_millis(100)).await?;
    client.debug_sleep(Duration::from_millis(200)).await?;
    client.debug_sleep(Duration::from_millis(200)).await?;

    let report: Vec<(String, u32, u32, u32)> = client.latency_latest().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn latency_reset() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    let report: Vec<(u32, u32)> = client.latency_history(LatencyHistoryEvent::Command).await?;
    assert_eq!(0, report.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn lolwut() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.lolwut(Default::default()).await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn memory_doctor() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.memory_doctor().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
async fn memory_help() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.memory_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn memory_malloc_stats() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.memory_malloc_stats().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[tokio::test]
#[serial]
async fn memory_purge() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.memory_purge().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn memory_stats() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let _memory_stats = client.memory_stats().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn memory_usage() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let size = client
        .memory_usage("key", Default::default())
        .await?
        .unwrap();
    assert!(size > 0);

    let size = client.memory_usage("unknown", Default::default()).await?;
    assert_eq!(None, size);

    client.set("key", "value").await?;
    let size = client
        .memory_usage("key", MemoryUsageOptions::default().samples(5))
        .await?
        .unwrap();
    assert!(size > 0);

    Ok(())
}

#[tokio::test]
async fn module_help() -> Result<()> {
    let client = get_test_client().await?;

    let result: Vec<String> = client.module_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn module_list() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let modules: Vec<ModuleInfo> = client.module_list().await?;
    assert_eq!(5, modules.len());

    Ok(())
}

#[tokio::test]
#[serial]
async fn module_unload_and_loadex() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // Only the error paths are exercised: unloading one of the bundled modules
    // would break every search, json and probabilistic test that follows.
    let result = client.module_unload("nosuchmodule").await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    let result = client
        .module_loadex("/nonexistent/module.so", ModuleLoadexOptions::default())
        .await;
    assert!(matches!(
        result.unwrap_err().kind(),
        ErrorKind::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        })
    ));

    // The five bundled modules are still there.
    let modules: Vec<ModuleInfo> = client.module_list().await?;
    assert_eq!(5, modules.len());

    Ok(())
}

#[test]
fn module_loadex_args() {
    let cmd = TestClient
        .module_loadex("/path/mod.so", ModuleLoadexOptions::default())
        .command;
    assert_eq!("MODULE LOADEX /path/mod.so", cmd.to_string());

    // CONFIG is repeated once per name/value pair, and ARGS closes the command.
    let cmd = TestClient
        .module_loadex(
            "/path/mod.so",
            ModuleLoadexOptions::default()
                .config("timeout", "100")
                .config("mode", "fast")
                .args(["arg1", "arg2"]),
        )
        .command;
    assert_eq!(
        "MODULE LOADEX /path/mod.so CONFIG timeout 100 CONFIG mode fast ARGS arg1 arg2",
        cmd.to_string()
    );
}

#[tokio::test]
#[serial]
async fn monitor() -> Result<()> {
    let client = get_exclusive_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let client2 = get_test_client().await?;
    client2.select(2).await?;

    let mut monitor_stream = client.monitor().await?;

    spawn(async move {
        async fn calls(client: &Client) -> Result<()> {
            client.set("key", "value1").await?;
            client.set("key", "value2").await?;
            client.set("key", "value3").await?;

            Ok(())
        }

        let _result = calls(&client2).await;
    });

    // MONITOR reports every command served by the instance: commands issued by
    // connections other than client2 are skipped.
    let mut seen = 0;
    while seen < 3 {
        let result = monitor_stream
            .next()
            .await
            .ok_or_else(|| ErrorKind::Client(ClientError::Unexpected))?;
        if result.database != 2 || result.command != "SET" {
            continue;
        }
        assert!(result.unix_timestamp_millis > 0.0);
        assert_eq!(2, result.command_args.len());
        seen += 1;
    }

    // RESET is the only command allowed during a MONITOR session
    let result: Result<String> = client.get("key").await;
    assert!(result.is_err());

    monitor_stream.close().await?;

    client.select(2).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value3", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn auto_remonitor() -> Result<()> {
    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = get_exclusive_test_client_with_config(config).await?;
    client.flushdb(FlushingMode::Sync).await?;

    let client2 = get_test_client().await?;
    client2.select(2).await?;

    let client_id = client.client_id().await?;
    let mut on_reconnect = client.on_reconnect();

    let mut monitor_stream = client.monitor().await?;

    client2
        .client_kill(ClientKillOptions::default().id(client_id))
        .await?;

    // wait for reconnection before monitoring
    on_reconnect.recv().await.unwrap();

    spawn(async move {
        async fn calls(client: &Client) -> Result<()> {
            client.set("key", "value1").await?;
            client.set("key", "value2").await?;
            client.set("key", "value3").await?;

            Ok(())
        }

        let _result = calls(&client2).await;
    });

    // MONITOR reports every command served by the instance: commands issued by
    // connections other than client2 are skipped.
    let mut seen = 0;
    while seen < 3 {
        let result = monitor_stream
            .next()
            .await
            .ok_or_else(|| ErrorKind::Client(ClientError::Unexpected))?;
        if result.database != 2 || result.command != "SET" {
            continue;
        }
        assert!(result.unix_timestamp_millis > 0.0);
        assert_eq!(2, result.command_args.len());
        seen += 1;
    }

    monitor_stream.close().await?;

    client.select(2).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value3", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn replicaof() -> Result<()> {
    let client = get_test_client().await?;

    client
        .replicaof(ReplicaOfOptions::master("127.0.0.1", 6379))
        .await?;
    client.replicaof(ReplicaOfOptions::no_one()).await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn role() -> Result<()> {
    let client = get_test_client().await?;

    let role_result = client.role().await?;
    tracing::debug!("role_result: {role_result:?}");
    assert!(matches!(
        role_result,
        RoleResult::Master {
            master_replication_offset: _,
            replica_infos: _
        }
    ));

    client
        .replicaof(ReplicaOfOptions::master("127.0.0.1", 6379))
        .await?;

    let role_result = client.role().await?;
    tracing::debug!("role_result: {role_result:?}");
    assert!(matches!(
        role_result,
        RoleResult::Replica {
            master_ip: _,
            master_port: _,
            state: _,
            amount_data_received: _
        }
    ));

    client.replicaof(ReplicaOfOptions::no_one()).await?;

    let sentinel_client = get_sentinel_test_client().await?;
    let role_result = sentinel_client.role().await?;
    tracing::debug!("role_result: {role_result:?}");
    assert!(matches!(
        role_result,
        RoleResult::Sentinel {
            master_names
        } if master_names == vec!["myservice".to_owned()]
    ));

    Ok(())
}

#[tokio::test]
#[serial]
async fn save() -> Result<()> {
    let client = get_test_client().await?;

    client.save().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn slowlog_get() -> Result<()> {
    let client = get_test_client().await?;

    let _entries = client.slowlog_get(SlowLogGetOptions::default()).await?;
    let _entries = client.slowlog_get(SlowLogGetOptions::count(2)).await?;

    Ok(())
}

#[tokio::test]
async fn slowlog_help() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.slowlog_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));

    Ok(())
}

#[tokio::test]
#[serial]
async fn slowlog_len() -> Result<()> {
    let client = get_test_client().await?;

    let _len = client.slowlog_len().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn slowlog_reset() -> Result<()> {
    let client = get_test_client().await?;

    client.slowlog_reset().await?;
    let len = client.slowlog_len().await?;
    assert_eq!(0, len);

    Ok(())
}

#[tokio::test]
#[serial]
async fn swapdb() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.select(1).await?;
    client.set("key", "value").await?;

    client.swapdb(0, 1).await?;

    client.select(0).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[tokio::test]
#[serial]
async fn time() -> Result<()> {
    let client = get_test_client().await?;

    let (_unix_timestamp, _microseconds) = client.time().await?;

    Ok(())
}

/// `SHUTDOWN [NOSAVE | SAVE] [NOW] [FORCE] [ABORT]` takes the server down, and
/// it exits cleanly, so the container's restart-on-failure policy would not
/// bring it back and the rest of the suite would run against nothing. `ABORT`
/// is the one form that leaves the server up: it cancels a shutdown in
/// progress, and answers an error when there is none — which is still the
/// server accepting the command and reading its flag.
#[tokio::test]
#[serial]
async fn shutdown_abort() -> Result<()> {
    let client = get_test_client().await?;

    let result = client.shutdown(ShutdownOptions::default().abort()).await;

    let error = result.unwrap_err();
    let ErrorKind::Redis(e) = error.kind() else {
        panic!("expected the server to report that nothing is shutting down: {error:?}");
    };
    assert!(e.description.contains("No shutdown in progress"));

    client.ping::<()>(()).await?;

    Ok(())
}

/// The forms that do take the server down can only have their wire form
/// checked, against the syntax published for `SHUTDOWN`.
#[test]
fn hotkeys_start_args() {
    let cmd = TestClient
        .hotkeys_start(
            [HotKeysMetric::Cpu, HotKeysMetric::Net],
            HotKeysStartOptions::default(),
        )
        .command;
    assert_eq!("HOTKEYS START METRICS 2 CPU NET", cmd.to_string());

    // `SLOTS` carries its own count and is only accepted in cluster mode, which
    // the integration test cannot reach.
    let cmd = TestClient
        .hotkeys_start(
            [HotKeysMetric::Cpu],
            HotKeysStartOptions::default()
                .count(5)
                .duration(60)
                .sample(10)
                .slots([0, 100, 16383]),
        )
        .command;
    assert_eq!(
        "HOTKEYS START METRICS 1 CPU COUNT 5 DURATION 60 SAMPLE 10 SLOTS 3 0 100 16383",
        cmd.to_string()
    );

    // An empty slot list still emits the clause, so the server explains the
    // mistake rather than the client silently tracking everything.
    let cmd = TestClient
        .hotkeys_start(
            [HotKeysMetric::Cpu],
            HotKeysStartOptions::default().slots([]),
        )
        .command;
    assert_eq!("HOTKEYS START METRICS 1 CPU SLOTS 0", cmd.to_string());
}

#[test]
fn shutdown_command() {
    let cmd = TestClient.shutdown(ShutdownOptions::default()).command;
    assert_eq!("SHUTDOWN", cmd.to_string());

    let cmd = TestClient
        .shutdown(ShutdownOptions::default().save(false).now().force())
        .command;
    assert_eq!("SHUTDOWN NOSAVE NOW FORCE", cmd.to_string());

    let cmd = TestClient
        .shutdown(ShutdownOptions::default().save(true))
        .command;
    assert_eq!("SHUTDOWN SAVE", cmd.to_string());
}

/// `COMMAND LIST FILTERBY MODULE module-name`. The test server loads the search,
/// json, timeseries and bloom modules, so filtering by one of them returns that
/// module's commands and nothing from the core.
#[tokio::test]
#[serial]
async fn command_list_filter_by_module_name() -> Result<()> {
    let client = get_test_client().await?;

    let module_commands: Vec<String> = client
        .command_list(CommandListOptions::default().filter_by_module_name("timeseries"))
        .await?;
    assert!(!module_commands.is_empty());
    assert!(
        module_commands
            .iter()
            .all(|name| name.starts_with("ts.") || name.starts_with("timeseries."))
    );
    assert!(module_commands.contains(&"ts.add".to_owned()));

    let module_commands: Vec<String> = client
        .command_list(CommandListOptions::default().filter_by_module_name("nosuchmodule"))
        .await?;
    assert!(module_commands.is_empty());

    Ok(())
}

/// `LOLWUT [VERSION version]` plus the version's own trailing arguments, which
/// for version 5 are the canvas width and height.
#[tokio::test]
#[serial]
async fn lolwut_version_and_optional_args() -> Result<()> {
    let client = get_test_client().await?;

    let small = client
        .lolwut(
            LolWutOptions::default()
                .version(5)
                .optional_arg(5)
                .optional_arg(5),
        )
        .await?;
    let large = client
        .lolwut(
            LolWutOptions::default()
                .version(5)
                .optional_arg(30)
                .optional_arg(30),
        )
        .await?;

    assert!(!small.is_empty());
    assert!(large.lines().count() > small.lines().count());

    Ok(())
}

/// `FAILOVER [TO host port [FORCE]] [ABORT] [TIMEOUT milliseconds]`. TO promotes
/// a designated replica, so it cannot be sent against the shared test server;
/// the wire form is asserted instead.
#[test]
fn failover_to_args() {
    let cmd = TestClient
        .failover(FailOverOptions::default().to("127.0.0.1", 6379))
        .command;
    assert_eq!("FAILOVER TO 127.0.0.1 6379", cmd.to_string());

    let cmd = TestClient
        .failover(
            FailOverOptions::default()
                .to("127.0.0.1", 6379)
                .force()
                .timeout(1000),
        )
        .command;
    assert_eq!(
        "FAILOVER TO 127.0.0.1 6379 FORCE TIMEOUT 1000",
        cmd.to_string()
    );
}
