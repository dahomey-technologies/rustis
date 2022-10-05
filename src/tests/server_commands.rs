use crate::{
    resp::{BulkString, Value},
    tests::get_test_client,
    AclCatOptions, AclDryRunOptions, AclGenPassOptions, AclLogOptions, ClientInfo,
    ConnectionCommands, Error, FlushingMode, Result, ServerCommands, StringCommands,
};
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
    assert!(dangerous_commands.contains(&"flushdb".to_owned()));

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
        "This user has no permissions to run the 'get' command",
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
    // default `commands` rule
    assert!(
        matches!(rules.get("commands"), Some(Value::BulkString(BulkString::Binary(rule))) if rule == b"-@all")
    );

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
    assert!(
        matches!(result, Err(Error::Redis(e)) if e.starts_with("ERR This Redis instance is not configured to use an ACL file."))
    );

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
    assert!(
        matches!(logs[0].get("reason"), Some(Value::BulkString(BulkString::Binary(reason))) if reason == b"auth")
    );
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
    assert!(
        matches!(result, Err(Error::Redis(e)) if e.starts_with("ERR This Redis instance is not configured to use an ACL file."))
    );

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
    assert!(matches!(result, Err(Error::Redis(e)) if e.starts_with("NOPERM")));

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

    client.acl_setuser("foo", ["on", ">pwd", "+ACL|WHOAMI"]).await?;
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

    let _command_infos = client.command_info("MGET").await?;

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
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

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
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client0.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client1.get("key1").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

    let value: Value = client1.get("key2").await?;
    assert!(matches!(value, Value::BulkString(BulkString::Nil)));

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
