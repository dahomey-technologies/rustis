use crate::{
    Result,
    commands::{
        ConnectionCommands, ExpireOption, FlushingMode, GenericCommands, ListCommands,
        MigrateOptions, MigrateResult, RestoreOptions, ScanOptions, ServerCommands, SetCommands,
        SortOptions, StringCommands,
    },
    resp::Value,
    tests::{TestClient, get_sentinel_master_test_client, get_test_client},
};
use serial_test::serial;
use std::{collections::HashSet, time::SystemTime};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn copy() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    // cleanup
    client0.del(["key", "key1"]).await?;
    client1.del(["key", "key1"]).await?;

    client0.set("key", "value").await?;

    let result = client0.copy("key", "key1", None, false).await?;
    assert!(result);
    let value: String = client0.get("key1").await?;
    assert_eq!("value", value);

    client0.set("key", "new_value").await?;
    let result = client0.copy("key", "key1", None, false).await?;
    assert!(!result);
    let value: String = client0.get("key1").await?;
    assert_eq!("value", value);

    let result = client0.copy("key", "key1", None, true).await?;
    assert!(result);
    let value: String = client0.get("key1").await?;
    assert_eq!("new_value", value);

    let result = client0.copy("key", "key", Some(1), false).await?;
    assert!(result);
    let value: String = client1.get("key").await?;
    assert_eq!("new_value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn del() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key1", "value1").await?;
    client.set("key2", "value2").await?;
    client.set("key3", "value3").await?;

    let deleted = client.del("key1").await?;
    assert_eq!(1, deleted);

    let deleted = client.del(["key1", "key2", "key3"]).await?;
    assert_eq!(2, deleted);

    let deleted = client.del("key1").await?;
    assert_eq!(0, deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn dump() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "value").await?;

    let dump = client.dump("key").await?;
    assert!(!dump.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn exists() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client.set("key1", "value1").await?;

    let result = client.exists("key1").await?;
    assert_eq!(1, result);

    let result = client.exists(["key1", "key2"]).await?;
    assert_eq!(1, result);

    let result = client.exists("key2").await?;
    assert_eq!(0, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expire() -> Result<()> {
    let client = get_test_client().await?;

    // no option
    client.set("key", "value").await?;
    let result = client.expire("key", 10, None).await?;
    assert!(result);
    assert_eq!(10, client.ttl("key").await?);

    // xx
    client.set("key", "value").await?;
    let result = client.expire("key", 10, ExpireOption::Xx).await?;
    assert!(!result);
    assert_eq!(-1, client.ttl("key").await?);

    // nx
    let result = client.expire("key", 10, ExpireOption::Nx).await?;
    assert!(result);
    assert_eq!(10, client.ttl("key").await?);

    // gt
    let result = client.expire("key", 5, ExpireOption::Gt).await?;
    assert!(!result);
    assert_eq!(10, client.ttl("key").await?);
    let result = client.expire("key", 15, ExpireOption::Gt).await?;
    assert!(result);
    assert_eq!(15, client.ttl("key").await?);

    // lt
    let result = client.expire("key", 20, ExpireOption::Lt).await?;
    assert!(!result);
    assert_eq!(15, client.ttl("key").await?);
    let result = client.expire("key", 5, ExpireOption::Lt).await?;
    assert!(result);
    assert_eq!(5, client.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expireat() -> Result<()> {
    let client = get_test_client().await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();

    // no option
    client.set("key", "value").await?;
    let result = client.expireat("key", now + 10, None).await?;
    assert!(result);
    let ttl = client.ttl("key").await?;
    assert!((9..=10).contains(&ttl));

    // xx
    client.set("key", "value").await?;
    let result = client.expireat("key", now + 10, ExpireOption::Xx).await?;
    assert!(!result);
    assert_eq!(-1, client.ttl("key").await?);

    // nx
    let result = client.expireat("key", now + 10, ExpireOption::Nx).await?;
    assert!(result);
    assert!((9..=10).contains(&ttl));

    // gt
    let result = client.expireat("key", now + 5, ExpireOption::Gt).await?;
    assert!(!result);
    assert!((9..=10).contains(&ttl));
    let result = client.expireat("key", now + 15, ExpireOption::Gt).await?;
    assert!(result);
    let ttl = client.ttl("key").await?;
    assert!((14..=15).contains(&ttl));

    // lt
    let result = client.expireat("key", now + 20, ExpireOption::Lt).await?;
    assert!(!result);
    let ttl = client.ttl("key").await?;
    assert!((14..=15).contains(&ttl));
    let result = client.expireat("key", now + 5, ExpireOption::Lt).await?;
    assert!(result);
    let ttl = client.ttl("key").await?;
    assert!((4..=5).contains(&ttl));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn expiretime() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "value").await?;
    assert!(client.expireat("key", 33177117420, None).await?);
    let time = client.expiretime("key").await?;
    assert_eq!(time, 33177117420);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn keys() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;
    client
        .mset([
            ("firstname", "Jack"),
            ("lastname", "Stuntman"),
            ("age", "35"),
        ])
        .await?;

    let keys: HashSet<String> = client.keys("*name*").await?;
    assert_eq!(2, keys.len());
    assert!(keys.contains("firstname"));
    assert!(keys.contains("lastname"));

    let keys: HashSet<String> = client.keys("a??").await?;
    assert_eq!(1, keys.len());
    assert!(keys.contains("age"));

    let keys: HashSet<String> = client.keys("*").await?;
    assert_eq!(3, keys.len());
    assert!(keys.contains("firstname"));
    assert!(keys.contains("lastname"));
    assert!(keys.contains("age"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn move_() -> Result<()> {
    let client0 = get_test_client().await?;
    let client1 = get_test_client().await?;
    client1.select(1).await?;

    // cleanup
    client0.del("key").await?;
    client1.del("key").await?;

    client0.set("key", "value").await?;
    client0.move_("key", 1).await?;
    assert_eq!(0, client0.exists("key").await?);
    assert_eq!(1, client1.exists("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_encoding() -> Result<()> {
    let client = get_test_client().await?;

    client.del(["key1", "key2", "unknown"]).await?;
    client.set("key1", "value").await?;
    client.set("key2", "12").await?;

    let encoding: String = client.object_encoding("key1").await?;
    assert_eq!("embstr", encoding);

    let encoding: String = client.object_encoding("key2").await?;
    assert_eq!("int", encoding);

    let encoding: String = client.object_encoding("unknown").await?;
    assert_eq!("", encoding);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_freq() -> Result<()> {
    let client = get_test_client().await?;

    client.del("key").await?;
    client.set("key", "value").await?;

    let frequency = client.object_freq("key").await;
    // ERR An LFU maxmemory policy is not selected, access frequency not tracked.
    assert!(frequency.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
async fn object_help() -> Result<()> {
    let client = get_test_client().await?;
    let result: Vec<String> = client.object_help().await?;
    assert!(result.iter().any(|e| e == "HELP"));
    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_idle_time() -> Result<()> {
    let client = get_test_client().await?;

    client.del("key").await?;
    client.set("key", "value").await?;

    // A key written a moment ago reads back as idle for 0 seconds — or for 1,
    // which is not a bug. Redis stamps the object with a cached LRU clock of
    // 1-second resolution that `serverCron` refreshes, so a tick landing between
    // the SET and the OBJECT IDLETIME yields 1. Measured at ~1 in 4000 pairs on
    // an idle server, and more often when the round trip is slow. Asserting `< 1`
    // asserts exact equality with 0, which Redis does not guarantee; `<= 1` still
    // proves the key is fresh rather than stale.
    let idle_time = client.object_idle_time("key").await?;
    assert!(
        idle_time <= 1,
        "a just-written key reported {idle_time}s idle"
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn object_refcount() -> Result<()> {
    let client = get_test_client().await?;

    client.del("key").await?;
    client.set("key", "value").await?;

    let refcount = client.object_refcount("key").await?;
    assert_eq!(1, refcount);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn persist() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "value").await?;
    assert!(client.expire("key", 10, None).await?);
    assert_eq!(10, client.ttl("key").await?);
    assert!(client.persist("key").await?);
    assert_eq!(-1, client.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpire() -> Result<()> {
    let client = get_test_client().await?;

    // no option
    client.set("key", "value").await?;
    let result = client.pexpire("key", 10000, None).await?;
    assert!(result);
    assert_eq!(10, client.ttl("key").await?);

    // xx
    client.set("key", "value").await?;
    let result = client.pexpire("key", 10000, ExpireOption::Xx).await?;
    assert!(!result);
    assert_eq!(-1, client.ttl("key").await?);

    // nx
    let result = client.pexpire("key", 10000, ExpireOption::Nx).await?;
    assert!(result);
    assert_eq!(10, client.ttl("key").await?);

    // gt
    let result = client.pexpire("key", 5000, ExpireOption::Gt).await?;
    assert!(!result);
    assert_eq!(10, client.ttl("key").await?);
    let result = client.pexpire("key", 15000, ExpireOption::Gt).await?;
    assert!(result);
    assert_eq!(15, client.ttl("key").await?);

    // lt
    let result = client.pexpire("key", 20000, ExpireOption::Lt).await?;
    assert!(!result);
    assert_eq!(15, client.ttl("key").await?);
    let result = client.pexpire("key", 5000, ExpireOption::Lt).await?;
    assert!(result);
    assert_eq!(5, client.ttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpireat() -> Result<()> {
    let client = get_test_client().await?;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis() as u64;

    // no option
    client.set("key", "value").await?;
    let result = client.pexpireat("key", now + 10000, None).await?;
    assert!(result);
    assert!(10000 >= client.pttl("key").await?);

    // xx
    client.set("key", "value").await?;
    let result = client
        .pexpireat("key", now + 10000, ExpireOption::Xx)
        .await?;
    assert!(!result);
    assert_eq!(-1, client.pttl("key").await?);

    // nx
    let result = client
        .pexpireat("key", now + 10000, ExpireOption::Nx)
        .await?;
    assert!(result);
    assert!(10000 >= client.pttl("key").await?);

    // gt
    let result = client
        .pexpireat("key", now + 5000, ExpireOption::Gt)
        .await?;
    assert!(!result);
    assert!(10000 >= client.pttl("key").await?);
    let result = client
        .pexpireat("key", now + 15000, ExpireOption::Gt)
        .await?;
    assert!(result);
    assert!(15000 >= client.pttl("key").await?);

    // lt
    let result = client
        .pexpireat("key", now + 20000, ExpireOption::Lt)
        .await?;
    assert!(!result);
    assert!(20000 >= client.pttl("key").await?);
    let result = client
        .pexpireat("key", now + 5000, ExpireOption::Lt)
        .await?;
    assert!(result);
    assert!(5000 >= client.pttl("key").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pexpiretime() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "value").await?;
    assert!(client.pexpireat("key", 33177117420000, None).await?);
    let time = client.pexpiretime("key").await?;
    assert_eq!(time, 33177117420000);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn randomkey() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;
    client.set("key1", "value1").await?;
    client.set("key2", "value2").await?;
    client.set("key3", "value3").await?;

    let key: String = client.randomkey().await?;
    assert!(["key1", "key2", "key3"].contains(&key.as_str()));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rename() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;
    client.set("key1", "value1").await?;

    client.rename("key1", "key2").await?;
    let value: Value = client.get("key1").await?;
    assert!(matches!(value, Value::Null));
    let value: String = client.get("key2").await?;
    assert_eq!("value1", value);

    let result = client.rename("unknown", "key2").await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn renamenx() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;
    client.set("key1", "value1").await?;

    let success = client.renamenx("key1", "key2").await?;
    assert!(success);

    client.set("key1", "value1").await?;
    let success = client.renamenx("key1", "key2").await?;
    assert!(!success);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn restore() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "value").await?;

    let dump = client.dump("key").await?;
    client.del("key").await?;
    client
        .restore("key", 0, &dump, RestoreOptions::default())
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn scan() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;

    client.set("key1", "value").await?;
    client.set("key2", "value").await?;
    client.set("key3", "value").await?;

    let keys: (u64, HashSet<String>) = client.scan(0, ScanOptions::default()).await?;
    assert_eq!(3, keys.1.len());
    assert!(keys.1.contains("key1"));
    assert!(keys.1.contains("key2"));
    assert!(keys.1.contains("key3"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sort() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;

    client
        .rpush("key", ["member3", "member1", "member2"])
        .await?;

    let values: Vec<String> = client.sort("key", SortOptions::default().alpha()).await?;
    assert_eq!(3, values.len());
    assert_eq!("member1".to_owned(), values[0]);
    assert_eq!("member2".to_owned(), values[1]);
    assert_eq!("member3".to_owned(), values[2]);

    let len = client
        .sort_and_store("key", "out", SortOptions::default().alpha())
        .await?;
    assert_eq!(3, len);

    let values: Vec<String> = client.lrange("out", 0, -1).await?;
    assert_eq!(3, values.len());
    assert_eq!("member1".to_owned(), values[0]);
    assert_eq!("member2".to_owned(), values[1]);
    assert_eq!("member3".to_owned(), values[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn sort_readonly() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;

    client
        .rpush("key", ["member3", "member1", "member2"])
        .await?;

    let values: Vec<String> = client
        .sort_readonly("key", SortOptions::default().alpha())
        .await?;
    assert_eq!(
        vec![
            "member1".to_owned(),
            "member2".to_owned(),
            "member3".to_owned()
        ],
        values
    );

    let values: Vec<String> = client
        .sort_readonly("key", SortOptions::default().alpha().limit(1, 1))
        .await?;
    assert_eq!(vec!["member2".to_owned()], values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn waitaof() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "value").await?;

    // The test server has no AOF replica, so asking for one times out and
    // answers the counts reached instead of failing.
    let (num_local, num_replicas) = client.waitaof(0, 0, 0).await?;
    assert_eq!(0, num_replicas);
    assert!(num_local <= 1);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn touch() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key1", "Hello").await?;
    client.set("key2", "World").await?;

    let num_keys = client.touch(["key1", "key2"]).await?;
    assert_eq!(2, num_keys);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn type_() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["key1", "key2", "key3"]).await?;

    client.set("key1", "value").await?;
    client.lpush("key2", "value").await?;
    client.sadd("key3", "value").await?;

    let result: String = client.type_("key1").await?;
    assert_eq!(&result, "string");

    let result: String = client.type_("key2").await?;
    assert_eq!(&result, "list");

    let result: String = client.type_("key3").await?;
    assert_eq!(&result, "set");

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn unlink() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key1", "value1").await?;
    client.set("key2", "value2").await?;
    client.set("key3", "value3").await?;

    let unlinked = client.unlink("key1").await?;
    assert_eq!(1, unlinked);

    let unlinked = client.unlink(["key1", "key2", "key3"]).await?;
    assert_eq!(2, unlinked);

    Ok(())
}

/// `RESTORE key ttl serialized-value [REPLACE] [ABSTTL] [IDLETIME seconds]
/// [FREQ frequency]`. REPLACE lets the restore overwrite a live key, and ABSTTL
/// reads the ttl argument as a Unix time in milliseconds instead of a delay.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn restore_replace_and_abs_ttl() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let dump = client.dump("key").await?;

    // Without REPLACE the server refuses to overwrite the live key.
    let result = client
        .restore("key", 0, &dump, RestoreOptions::default())
        .await;
    assert!(result.is_err());

    client.set("key", "other").await?;
    client
        .restore("key", 0, &dump, RestoreOptions::default().replace())
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    // ABSTTL reads the ttl as a Unix time in milliseconds.
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    client
        .restore(
            "key",
            now_ms + 100_000,
            &dump,
            RestoreOptions::default().replace().abs_ttl().idle_time(0),
        )
        .await?;
    let ttl = client.ttl("key").await?;
    assert!(ttl > 90 && ttl <= 100);

    Ok(())
}

/// `MIGRATE host port key destination-db timeout [COPY] [REPLACE]
/// [AUTH password | AUTH2 username password] [KEYS key ...]`. The destination is
/// the sentinel master of `redis/docker-compose.yml`, reachable from the source
/// container by name; a server cannot migrate to itself, since it would block
/// waiting for its own reply.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn migrate_replace() -> Result<()> {
    let client = get_test_client().await?;
    let destination = get_sentinel_master_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;
    destination.del("migrated").await?;

    client.set("migrated", "first").await?;
    let result = client
        .migrate(
            "redis-sentinel-master",
            6381,
            "migrated",
            0,
            1000,
            MigrateOptions::default(),
        )
        .await?;
    assert!(matches!(result, MigrateResult::Ok));

    // The key now exists at the destination, so a second migration is refused.
    client.set("migrated", "second").await?;
    let result = client
        .migrate(
            "redis-sentinel-master",
            6381,
            "migrated",
            0,
            1000,
            MigrateOptions::default(),
        )
        .await;
    assert!(result.is_err());

    // REPLACE overwrites it, and COPY keeps the source key in place.
    let result = client
        .migrate(
            "redis-sentinel-master",
            6381,
            "migrated",
            0,
            1000,
            MigrateOptions::default().replace().copy(),
        )
        .await?;
    assert!(matches!(result, MigrateResult::Ok));

    let value: String = destination.get("migrated").await?;
    assert_eq!("second", value);
    let value: String = client.get("migrated").await?;
    assert_eq!("second", value);

    destination.del("migrated").await?;

    Ok(())
}

/// `AUTH password` and `AUTH2 username password` are the two authentication
/// forms of MIGRATE. The test servers need no password, so the wire form is
/// asserted instead.
#[test]
fn migrate_auth_args() {
    let cmd = TestClient
        .migrate(
            "host",
            6379,
            "key",
            0,
            1000,
            MigrateOptions::default().auth("password"),
        )
        .command;
    assert_eq!(
        "MIGRATE host 6379 key 0 1000 AUTH password",
        cmd.to_string()
    );

    let cmd = TestClient
        .migrate(
            "host",
            6379,
            "key",
            0,
            1000,
            MigrateOptions::default().auth2("username", "password"),
        )
        .command;
    assert_eq!(
        "MIGRATE host 6379 key 0 1000 AUTH2 username password",
        cmd.to_string()
    );

    // With KEYS the single-key slot is an empty string and the keys follow.
    let cmd = TestClient
        .migrate(
            "host",
            6379,
            "",
            0,
            1000,
            MigrateOptions::default().replace().key("key1").key("key2"),
        )
        .command;
    assert_eq!(
        "MIGRATE host 6379  0 1000 REPLACE KEYS key1 key2",
        cmd.to_string()
    );
}
