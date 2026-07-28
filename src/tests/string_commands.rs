use crate::{
    Error, RedisError, RedisErrorKind, Result,
    commands::{
        DelexCondition, FlushingMode, GenericCommands, GetExOptions, IncrExOptions, LcsMatch,
        ServerCommands, SetCondition, SetExpiration, StringCommands,
    },
    resp::Value,
    tests::{TestClient, get_test_client},
};
use serial_test::serial;
use std::time::{Duration, SystemTime};

#[tokio::test]
#[serial]
async fn append() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let new_size = client.append("key", "12").await?;
    assert_eq!(7, new_size);

    let value: String = client.get("key").await?;
    assert_eq!("value12", value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn digest_and_delex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "hello").await?;

    // DIGEST returns the value's hash as a hex string; equal values hash equally.
    let digest: String = client.digest("key").await?;
    assert!(!digest.is_empty());
    client.set("key2", "hello").await?;
    let digest2: String = client.digest("key2").await?;
    assert_eq!(digest, digest2);

    // DELEX with a non-matching value must not delete.
    let deleted = client.delex("key", DelexCondition::IFEQ("world")).await?;
    assert_eq!(0, deleted);
    assert_eq!(1, client.exists("key").await?);

    // DELEX with a matching digest deletes.
    let deleted = client.delex("key", DelexCondition::IFDEQ(&digest)).await?;
    assert_eq!(1, deleted);
    assert_eq!(0, client.exists("key").await?);

    // DELEX without a condition deletes unconditionally.
    let deleted = client.delex("key2", None).await?;
    assert_eq!(1, deleted);

    // DELEX on a missing key returns 0.
    let deleted = client.delex("missing", None).await?;
    assert_eq!(0, deleted);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn decr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.decr("key").await?;
    assert_eq!(-1, value);

    client.set("key", "12").await?;

    let value = client.decr("key").await?;
    assert_eq!(11, value);

    client.set("key", "value").await?;

    let result = client.decr("key").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn decrby() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.decrby("key", 2).await?;
    assert_eq!(-2, value);

    client.set("key", "12").await?;

    let value = client.decrby("key", 2).await?;
    assert_eq!(10, value);

    client.set("key", "value").await?;

    let result = client.decrby("key", 2).await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn get_and_set() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn get_ex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn get_pex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Px(1000)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn get_exat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    let value: String = client.getex("key", GetExOptions::Exat(time)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn get_pxat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    let value: String = client.getex("key", GetExOptions::Pxat(time as u64)).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn get_persist() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.getex("key", GetExOptions::Ex(1)).await?;
    assert_eq!("value", value);

    let value: String = client.getex("key", GetExOptions::Persist).await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert_eq!(-1, ttl);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn getrange() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("mykey", "This is a string").await?;

    let value: String = client.getrange("mykey", 0, 3).await?;
    assert_eq!("This", value);
    let value: String = client.getrange("mykey", -3, -1).await?;
    assert_eq!("ing", value);
    let value: String = client.getrange("mykey", 0, -1).await?;
    assert_eq!("This is a string", value);
    let value: String = client.getrange("mykey", 10, 100).await?;
    assert_eq!("string", value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn getdel() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let value: String = client.getdel("key").await?;
    assert_eq!("value", value);

    // The key is gone, so a second call answers nil.
    let value: Value = client.getdel("key").await?;
    assert!(matches!(value, Value::Null));

    assert_eq!(0, client.exists("key").await?);

    Ok(())
}

#[tokio::test]
#[serial]
async fn getset() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;

    let value: String = client.getset("key", "newvalue").await?;
    assert_eq!("value", value);

    client.del("key").await?;

    let value: Value = client.getset("key", "newvalue").await?;
    assert!(matches!(value, Value::Null));

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn incr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.incr("key").await?;
    assert_eq!(1, value);

    client.set("key", "12").await?;

    let value = client.incr("key").await?;
    assert_eq!(13, value);

    client.set("key", "value").await?;

    let result = client.incr("key").await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn increx() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // Without an increment the key is created at 0 and bumped by 1.
    let (value, applied): (i64, i64) = client.increx("key", IncrExOptions::default()).await?;
    assert_eq!((1, 1), (value, applied));

    let (value, applied): (i64, i64) = client
        .increx("key", IncrExOptions::by_int(-10).ex(100))
        .await?;
    assert_eq!((-9, -10), (value, applied));
    assert_eq!(100, client.ttl("key").await?);

    // ENX leaves an existing TTL alone; the increment still lands.
    let (value, _): (i64, i64) = client
        .increx("key", IncrExOptions::by_int(1).ex(10).enx())
        .await?;
    assert_eq!(-8, value);
    assert_eq!(100, client.ttl("key").await?);

    // Out of bounds: the key is untouched and the applied increment is 0.
    client.set("bounded", 99).await?;
    let (value, applied): (i64, i64) = client
        .increx("bounded", IncrExOptions::by_int(5).ubound_int(100))
        .await?;
    assert_eq!((99, 0), (value, applied));

    // SATURATE caps at the bound instead, and reports the delta it did apply.
    let (value, applied): (i64, i64) = client
        .increx(
            "bounded",
            IncrExOptions::by_int(5).ubound_int(100).saturate(),
        )
        .await?;
    assert_eq!((100, 1), (value, applied));

    client.set("float", "1.5").await?;
    let (value, applied): (f64, f64) = client
        .increx("float", IncrExOptions::by_float(0.25).persist())
        .await?;
    assert_eq!((1.75, 0.25), (value, applied));

    Ok(())
}

#[test]
fn increx_args() {
    let cmd = TestClient
        .increx::<()>(
            "key",
            IncrExOptions::by_int(5)
                .lbound_int(0)
                .ubound_int(100)
                .saturate()
                .ex(60)
                .enx(),
        )
        .command;
    assert_eq!(
        "INCREX key BYINT 5 LBOUND 0 UBOUND 100 SATURATE EX 60 ENX",
        cmd.to_string()
    );

    let cmd = TestClient
        .increx::<()>(
            "key",
            IncrExOptions::by_float(0.5).lbound_float(-1.5).persist(),
        )
        .command;
    assert_eq!(
        "INCREX key BYFLOAT 0.5 LBOUND -1.5 PERSIST",
        cmd.to_string()
    );

    let cmd = TestClient
        .increx::<()>("key", IncrExOptions::default())
        .command;
    assert_eq!("INCREX key", cmd.to_string());
}

#[tokio::test]
#[serial]
async fn incrby() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    let value = client.incrby("key", 2).await?;
    assert_eq!(2, value);

    client.set("key", "12").await?;

    let value = client.incrby("key", 2).await?;
    assert_eq!(14, value);

    client.set("key", "value").await?;

    let result = client.incrby("key", 2).await;
    assert!(matches!(
        result,
        Err(Error::Redis(RedisError {
            kind: RedisErrorKind::Err,
            description: _
        }))
    ));

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn incrbyfloat() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del("key").await?;

    client.set("key", "10.50").await?;

    let value = client.incrbyfloat("key", 0.1).await?;
    assert_eq!(10.6, value);

    let value = client.incrbyfloat("key", -5f64).await?;
    assert_eq!(5.6, value);

    client.set("key", "5.0e3").await?;

    let value = client.incrbyfloat("key", 2.0e2f64).await?;
    assert_eq!(5200f64, value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn lcs() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del(["key1", "key2"]).await?;

    client
        .mset([("key1", "ohmytext"), ("key2", "mynewtext")])
        .await?;

    let result: String = client.lcs("key1", "key2").await?;
    assert_eq!("mytext", result);

    let result = client.lcs_len("key1", "key2").await?;
    assert_eq!(6, result);

    let result = client.lcs_idx("key1", "key2", None, false).await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(LcsMatch((4, 7), (5, 8), None), result.matches[0]);
    assert_eq!(LcsMatch((2, 3), (0, 1), None), result.matches[1]);

    let result = client.lcs_idx("key1", "key2", Some(4), false).await?;
    assert_eq!(6, result.len);
    assert_eq!(1, result.matches.len());
    assert_eq!(LcsMatch((4, 7), (5, 8), None), result.matches[0]);

    let result = client.lcs_idx("key1", "key2", None, true).await?;
    assert_eq!(6, result.len);
    assert_eq!(2, result.matches.len());
    assert_eq!(LcsMatch((4, 7), (5, 8), Some(4)), result.matches[0]);
    assert_eq!(LcsMatch((2, 3), (0, 1), Some(2)), result.matches[1]);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn mget_mset() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    let keys = ["key1", "key2", "key3", "key4"];

    // cleanup
    client.del(keys).await?;

    let items = [("key1", "value1"), ("key2", "value2"), ("key3", "value3")];
    client.mset(items).await?;

    let values: Vec<Option<String>> = client.mget(keys).await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn msetnx() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // cleanup
    client.del(["key1", "key2", "key3", "key4"]).await?;

    let success = client
        .msetnx([("key1", "value1"), ("key2", "value2"), ("key3", "value3")])
        .await?;
    assert!(success);

    let values: Vec<Option<String>> = client.mget(["key1", "key2", "key3", "key4"]).await?;
    assert_eq!(4, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));
    assert!(matches!(&values[2], Some(value) if value == "value3"));
    assert_eq!(values[3], None);

    let success = client
        .msetnx([("key1", "value1"), ("key4", "value4")])
        .await?;
    assert!(!success);

    let values: Vec<Option<String>> = client.mget(["key1", "key4"]).await?;
    assert_eq!(2, values.len());
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert_eq!(values[1], None);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn msetex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // plain multi-set, no condition, no expiration
    let success = client
        .msetex([("key1", "value1"), ("key2", "value2")], None, None)
        .await?;
    assert!(success);

    let values: Vec<Option<String>> = client.mget(["key1", "key2"]).await?;
    assert!(matches!(&values[0], Some(value) if value == "value1"));
    assert!(matches!(&values[1], Some(value) if value == "value2"));

    // with a shared expiration
    let success = client
        .msetex(
            [("key1", "value1"), ("key2", "value2")],
            None,
            Some(SetExpiration::Ex(100)),
        )
        .await?;
    assert!(success);
    let ttl = client.ttl("key1").await?;
    assert!(ttl > 0);

    // NX must fail when at least one key already exists
    let success = client
        .msetex([("key1", "other")], Some(SetCondition::NX), None)
        .await?;
    assert!(!success);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn psetex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.psetex("key", 1000, "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn set_with_options() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    // EX
    client
        .set_with_options("key", "value", None, Some(SetExpiration::Ex(1)))
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // PX
    client
        .set_with_options("key", "value", None, Some(SetExpiration::Px(1000)))
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // EXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs();
    client
        .set_with_options("key", "value", None, Some(SetExpiration::Exat(time)))
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // PXAT
    let time = SystemTime::now()
        .checked_add(Duration::from_secs(1))
        .unwrap()
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_millis();
    client
        .set_with_options("key", "value", None, Some(SetExpiration::Pxat(time as u64)))
        .await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    // NX
    client.del("key").await?;
    let result = client
        .set_with_options("key", "value", Some(SetCondition::NX), None)
        .await?;
    assert!(result);
    let result = client
        .set_with_options("key", "value", Some(SetCondition::NX), None)
        .await?;
    assert!(!result);

    // XX
    client.del("key").await?;
    let result = client
        .set_with_options("key", "value", Some(SetCondition::XX), None)
        .await?;
    assert!(!result);
    client.set("key", "value").await?;
    let result = client
        .set_with_options("key", "value", Some(SetCondition::XX), None)
        .await?;
    assert!(result);

    // GET
    client.del("key").await?;
    let result: Option<String> = client
        .set_get_with_options("key", "value", None, None)
        .await?;
    assert!(result.is_none());
    client.set("key", "value").await?;
    let result: String = client
        .set_get_with_options("key", "value1", None, None)
        .await?;
    assert_eq!("value", result);
    let value: String = client.get("key").await?;
    assert_eq!("value1", value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn set_with_options_conditionals() -> Result<()> {
    let client = get_test_client().await?;

    // IFEQ: set only if the current value equals the provided one.
    client.set("key", "value").await?;
    let result = client
        .set_with_options("key", "new", Some(SetCondition::IFEQ("value")), None)
        .await?;
    assert!(result);
    let value: String = client.get("key").await?;
    assert_eq!("new", value);
    let result = client
        .set_with_options("key", "other", Some(SetCondition::IFEQ("wrong")), None)
        .await?;
    assert!(!result);

    // IFNE: set only if the current value differs from the provided one.
    client.set("key", "value").await?;
    let result = client
        .set_with_options("key", "new", Some(SetCondition::IFNE("value")), None)
        .await?;
    assert!(!result);
    let result = client
        .set_with_options("key", "new", Some(SetCondition::IFNE("different")), None)
        .await?;
    assert!(result);

    // IFDEQ / IFDNE compare against the XXH3 digest of the current value. Without
    // a matching digest at hand, assert the tokens are accepted by the server and
    // behave as expected: a wrong digest never matches (IFDEQ fails, IFDNE
    // succeeds), with no protocol error.
    client.set("key", "value").await?;
    let wrong_digest = "0".repeat(16);
    let result = client
        .set_with_options("key", "new", Some(SetCondition::IFDEQ(&wrong_digest)), None)
        .await?;
    assert!(!result);
    let result = client
        .set_with_options("key", "new", Some(SetCondition::IFDNE(&wrong_digest)), None)
        .await?;
    assert!(result);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn setex() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.setex("key", 1, "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    let ttl = client.pttl("key").await?;
    assert!(ttl <= 1000);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn setnx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushall(FlushingMode::Sync).await?;

    let result = client.setnx("key", "value").await?;
    let value: String = client.get("key").await?;
    assert!(result);
    assert_eq!("value", value);

    let result = client.setnx("key", "value1").await?;
    assert!(!result);
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn setrange() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "Hello World").await?;

    let new_len = client.setrange("key", 6, "Redis").await?;
    assert_eq!(11, new_len);

    let value: String = client.get("key").await?;
    assert_eq!("Hello Redis", value);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn strlen() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "Hello World").await?;

    let len = client.strlen("key").await?;
    assert_eq!(11, len);

    let len = client.strlen("nonexisting").await?;
    assert_eq!(0, len);

    client.close().await?;

    Ok(())
}

#[tokio::test]
#[serial]
async fn substr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client.set("mykey", "This is a string").await?;

    let value: String = client.substr("mykey", 0, 3).await?;
    assert_eq!("This", value);
    let value: String = client.substr("mykey", -3, -1).await?;
    assert_eq!("ing", value);
    let value: String = client.substr("mykey", 0, -1).await?;
    assert_eq!("This is a string", value);
    let value: String = client.substr("mykey", 10, 100).await?;
    assert_eq!("string", value);

    client.close().await?;

    Ok(())
}

/// The expiration forms of INCREX the server lists for itself — `EX seconds`,
/// `PX milliseconds`, `EXAT unix-time-seconds`, `PXAT unix-time-milliseconds` —
/// plus UBOUND in BYFLOAT mode, where the bound is a floating-point number.
#[tokio::test]
#[serial]
async fn increx_expirations_and_float_bound() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let (_, _): (i64, i64) = client
        .increx("key", IncrExOptions::by_int(1).px(100_000))
        .await?;
    let ttl = client.pttl("key").await?;
    assert!(ttl > 90_000 && ttl <= 100_000);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let (_, _): (i64, i64) = client
        .increx("key", IncrExOptions::by_int(1).exat(now + 100))
        .await?;
    let ttl = client.ttl("key").await?;
    assert!(ttl > 90 && ttl <= 100);

    let (_, _): (i64, i64) = client
        .increx("key", IncrExOptions::by_int(1).pxat((now + 200) * 1000))
        .await?;
    let ttl = client.ttl("key").await?;
    assert!(ttl > 190 && ttl <= 200);

    // UBOUND in BYFLOAT mode: 1.5 + 0.75 would pass 2.0, so the operation is
    // rejected and reports a zero increment.
    client.set("float", "1.5").await?;
    let (value, applied): (f64, f64) = client
        .increx("float", IncrExOptions::by_float(0.75).ubound_float(2.0))
        .await?;
    assert_eq!((1.5, 0.0), (value, applied));

    // Under the bound it lands.
    let (value, applied): (f64, f64) = client
        .increx("float", IncrExOptions::by_float(0.25).ubound_float(2.0))
        .await?;
    assert_eq!((1.75, 0.25), (value, applied));

    Ok(())
}
