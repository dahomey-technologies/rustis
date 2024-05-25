use crate::{
    client::{Client, ReconnectionConfig},
    commands::{
        AclCatOptions, AclDryRunOptions, AclGenPassOptions, AclLogOptions, BlockingCommands,
        ClientInfo, ClientKillOptions, CommandDoc, CommandHistogram, CommandListOptions,
        ConnectionCommands, FailOverOptions, FlushingMode, InfoSection, LatencyHistoryEvent,
        MemoryUsageOptions, ModuleInfo, ModuleLoadOptions, ReplicaOfOptions, RoleResult,
        ServerCommands, SlowLogOptions, StringCommands,
    },
    resp::{cmd, Value},
    spawn,
    tests::{get_default_config, get_sentinel_test_client, get_test_client, get_test_client_with_config},
    Error, RedisError, RedisErrorKind, Result,
};
use futures_util::StreamExt;
use serial_test::serial;
use std::collections::{HashMap, HashSet};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_cat() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let categories: Vec<String> = client.acl_cat(AclCatOptions::default()).await?;
    assert!(categories.contains(&"dangerous".to_owned()));

    let dangerous_commands: HashSet<String> = client
        .acl_cat(AclCatOptions::default().category_name("dangerous"))
        .await?;
    assert!(dangerous_commands.contains("flushdb"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_dryrun() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.acl_setuser("VIRGINIA", ["+SET", "~*"]).await?;
    client
        .acl_dryrun(
            "VIRGINIA",
            "SET",
            AclDryRunOptions::default().arg("foo").arg("bar"),
        )
        .await?;
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_genpass() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let password: String = client.acl_genpass(AclGenPassOptions::default()).await?;
    assert_eq!(64, password.len());

    let password: String = client
        .acl_genpass(AclGenPassOptions::default().bits(32))
        .await?;
    assert_eq!(8, password.len());

    let password: String = client
        .acl_genpass(AclGenPassOptions::default().bits(5))
        .await?;
    assert_eq!(2, password.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_getuser() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.acl_setuser("foo", Vec::<String>::new()).await?;
    let rules: HashMap<String, Value> = client.acl_getuser("foo").await?;
    log::debug!("rules: {rules:?}");
    // default `commands` rule
    assert!(matches!(rules.get("commands"), Some(Value::BulkString(rule)) if rule == b"-@all"));

    client.acl_deluser("foo").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_list() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let users: Vec<String> = client.acl_list().await?;
    assert_eq!(1, users.len());
    assert!(users[0].starts_with("user default"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_load() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.acl_load().await;
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
async fn acl_log() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.auth(Some("someuser"), "wrongpassword").await;
    assert!(result.is_err());

    let logs: Vec<HashMap<String, Value>> =
        client.acl_log(AclLogOptions::default().count(1)).await?;
    assert_eq!(1, logs.len());
    assert!(matches!(logs[0].get("reason"), Some(Value::BulkString(reason)) if reason == b"auth"));
    let client_info: String = logs[0].get("client-info").unwrap().to_string();
    let client_info = ClientInfo::from_line(&client_info)?;
    assert_eq!("auth", client_info.cmd);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn acl_save() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.acl_save().await;
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
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::NoPerm,
            description: _
        }))
    ));

    client.acl_setuser("foo", ["~key"]).await?;
    let _rules: HashMap<String, Value> = client.acl_getuser("foo").await?;
    client.set("key", "value").await?;

    client.acl_deluser("foo").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn command() -> Result<()> {
    let client = get_test_client().await?;

    let _command_infos = client.command().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn command_info() -> Result<()> {
    let client = get_test_client().await?;

    let _command_infos = client.command_info("SORT").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn command_count() -> Result<()> {
    let client = get_test_client().await?;

    let command_infos = client.command().await?;
    let num_commands = client.command_count().await?;
    assert_eq!(command_infos.len(), num_commands);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn command_docs() -> Result<()> {
    let client = get_test_client().await?;

    let _command_docs: HashMap<String, CommandDoc> =
        client.command_docs(["XADD", "GET", "SET"]).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn config_resetstat() -> Result<()> {
    let client = get_test_client().await?;

    client.config_resetstat().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn config_rewrite() -> Result<()> {
    let client = get_test_client().await?;

    let _result = client.config_rewrite().await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn failover() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client.failover(FailOverOptions::default()).await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description == "FAILOVER requires connected replicas."
    ));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushdb() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    client0.set("key1", "value1").await?;
    client0.set("key2", "value2").await?;

    client1.set("key1", "value1").await?;
    client1.set("key2", "value2").await?;

    client0.flushdb(FlushingMode::Default).await?;

    let value: Value = client0.get("key1").await?;
    assert!(matches!(value, Value::Nil));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::Nil));

    let value: String = client1.get("key1").await?;
    assert_eq!("value1", value);

    let value: String = client1.get("key2").await?;
    assert_eq!("value2", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn flushall() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    client0.set("key1", "value1").await?;
    client0.set("key2", "value2").await?;

    client1.set("key1", "value1").await?;
    client1.set("key2", "value2").await?;

    client0.flushall(FlushingMode::Default).await?;

    let value: Value = client0.get("key1").await?;
    assert!(matches!(value, Value::Nil));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::Nil));

    let value: Value = client1.get("key1").await?;
    assert!(matches!(value, Value::Nil));

    let value: Value = client1.get("key2").await?;
    assert!(matches!(value, Value::Nil));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn info() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let info = client.info([]).await?;
    assert!(!info.is_empty());

    let info = client
        .info([InfoSection::Cpu, InfoSection::Clients])
        .await?;
    assert!(info.contains("# CPU"));
    assert!(info.contains("# Clients"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lastsave() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let lastsave = client.lastsave().await?;
    assert!(lastsave > 0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn latency_doctor() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.latency_doctor().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn latency_graph() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.1), None)
        .await?;
    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.2), None)
        .await?;
    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.2), None)
        .await?;

    let report = client.latency_graph(LatencyHistoryEvent::Command).await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn latency_history() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.1), None)
        .await?;
    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.2), None)
        .await?;
    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.2), None)
        .await?;

    let report: Vec<(u32, u32)> = client.latency_history(LatencyHistoryEvent::Command).await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn latency_latest() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .config_set(("latency-monitor-threshold", "50"))
        .await?;

    client.latency_reset([LatencyHistoryEvent::Command]).await?;

    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.1), None)
        .await?;
    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.2), None)
        .await?;
    client
        .send(cmd("DEBUG").arg("SLEEP").arg(0.2), None)
        .await?;

    let report: Vec<(String, u32, u32, u32)> = client.latency_latest().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lolwut() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.lolwut(Default::default()).await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn memory_doctor() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.memory_doctor().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn memory_malloc_stats() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let report = client.memory_malloc_stats().await?;
    assert!(!report.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn memory_purge() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.memory_purge().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn memory_stats() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let _memory_stats = client.memory_stats().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn module_list() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let modules: Vec<ModuleInfo> = client.module_list().await?;
    assert_eq!(0, modules.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn module_load() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client
        .module_load(
            "path",
            ModuleLoadOptions::default()
                .config("name", "value")
                .config("name2", "value2")
                .arg("arg1")
                .arg(23),
        )
        .await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description.starts_with("MODULE command not allowed.")
    ));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn module_unload() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result = client.module_unload("mymodule").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description
        })) if description.starts_with("MODULE command not allowed.")
    ));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn monitor() -> Result<()> {
    let client = get_test_client().await?;
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

    for _ in 0..3 {
        let result = monitor_stream
            .next()
            .await
            .ok_or_else(|| Error::Client("fail".to_owned()))?;
        assert!(result.unix_timestamp_millis > 0.0);
        assert_eq!(2, result.database);
        assert_eq!("SET", result.command);
        assert_eq!(2, result.command_args.len());
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn auto_remonitor() -> Result<()> {
    let mut config = get_default_config()?;
    config.reconnection = ReconnectionConfig::new_constant(0, 100);
    let client = get_test_client_with_config(config).await?;
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

    for _ in 0..3 {
        let result = monitor_stream
            .next()
            .await
            .ok_or_else(|| Error::Client("fail".to_owned()))?;

        assert!(result.unix_timestamp_millis > 0.0);
        assert_eq!(2, result.database);
        assert_eq!("SET", result.command);
        assert_eq!(2, result.command_args.len());
    }

    monitor_stream.close().await?;

    client.select(2).await?;
    let value: String = client.get("key").await?;
    assert_eq!("value3", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn replicaof() -> Result<()> {
    let client = get_test_client().await?;

    client
        .replicaof(ReplicaOfOptions::master("127.0.0.1", 6379))
        .await?;
    client.replicaof(ReplicaOfOptions::no_one()).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn role() -> Result<()> {
    let client = get_test_client().await?;

    let role_result = client.role().await?;
    log::debug!("role_result: {role_result:?}");
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
    log::debug!("role_result: {role_result:?}");
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
    log::debug!("role_result: {role_result:?}");
    assert!(matches!(
        role_result,
        RoleResult::Sentinel {
            master_names
        } if master_names == vec!["myservice".to_owned()]
    ));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn save() -> Result<()> {
    let client = get_test_client().await?;

    client.save().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn slowlog_get() -> Result<()> {
    let client = get_test_client().await?;

    let _entries = client.slowlog_get(SlowLogOptions::default()).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn slowlog_len() -> Result<()> {
    let client = get_test_client().await?;

    let _len = client.slowlog_len().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn slowlog_reset() -> Result<()> {
    let client = get_test_client().await?;

    client.slowlog_reset().await?;
    let len = client.slowlog_len().await?;
    assert_eq!(0, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn time() -> Result<()> {
    let client = get_test_client().await?;

    let (_unix_timestamp, _microseconds) = client.time().await?;

    Ok(())
}
