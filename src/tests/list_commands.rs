use crate::{
    resp::{BulkString, Value},
    spawn,
    tests::{get_test_client, sleep},
    FlushingMode, GenericCommands, LInsertWhere,
    LMoveWhere::Left,
    LMoveWhere::Right,
    ListCommands, Result, ServerCommands,
};
use serial_test::serial;
use std::time::Duration;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn blmove() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let element: String = client
        .blmove("mylist", "myotherlist", Right, Left, 0.0)
        .await?;
    assert_eq!("element3", element);

    let element: String = client
        .blmove("mylist", "myotherlist", Left, Right, 0.0)
        .await?;
    assert_eq!("element1", element);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(1, elements.len());
    assert_eq!("element2".to_string(), elements[0]);

    let elements: Vec<String> = client.lrange("myotherlist", 0, -1).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3".to_string(), elements[0]);
    assert_eq!("element1".to_string(), elements[1]);

    let element: Option<String> = client
        .blmove("uknown", "myotherlist", Right, Left, 0.01)
        .await?;
    assert_eq!(None, element);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let element: String = client
                .blmove("mylist", "myotherlist", Right, Left, 0.0)
                .await?;
            assert_eq!("element4", element);

            Ok(())
        }

        let _result = calls().await;
    });

    client.rpush("mylist", "element4").await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn blmpop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .lpush(
            "mylist",
            ["element1", "element2", "element3", "element4", "element5"],
        )
        .await?;

    let (key, elements): (String, Vec<String>) =
        client.blmpop(0.0, "mylist", Left, 5).await?.unwrap();
    assert_eq!("mylist", key);
    assert_eq!(5, elements.len());
    assert_eq!("element5".to_string(), elements[0]);
    assert_eq!("element4".to_string(), elements[1]);
    assert_eq!("element3".to_string(), elements[2]);
    assert_eq!("element2".to_string(), elements[3]);
    assert_eq!("element1".to_string(), elements[4]);

    let result: Option<(String, Vec<String>)> = client.blmpop(0.01, "unknown", Left, 1).await?;
    assert_eq!(None, result);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let (key, elements): (String, Vec<String>) =
                client.blmpop(0.0, "mylist", Left, 1).await?.unwrap();
            assert_eq!("mylist", key);
            assert_eq!(1, elements.len());
            assert_eq!("element6".to_string(), elements[0]);

            Ok(())
        }

        let _result = calls().await;
    });

    client.lpush("mylist", "element6").await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn blpop() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result: Option<(String, String)> = client.blpop(["list", "other"], 0.01).await?;
    assert_eq!(None, result);

    client.rpush("list", "element1").await?;
    let result: Option<(String, String)> = client.blpop(["list", "other"], 0.0).await?;
    assert_eq!(Some(("list".to_owned(), "element1".to_owned())), result);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let result: Option<(String, String)> = client.blpop("list", 0.0).await?;
            assert_eq!(Some(("list".to_owned(), "element2".to_owned())), result);

            Ok(())
        }

        let _result = calls().await;
    });

    client.rpush("list", "element2").await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn brpop() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    let result: Option<(String, String)> = client.brpop(["list", "other"], 0.01).await?;
    assert_eq!(None, result);

    client.lpush("list", "element1").await?;
    let result: Option<(String, String)> = client.brpop(["list", "other"], 0.0).await?;
    assert_eq!(Some(("list".to_owned(), "element1".to_owned())), result);

    spawn(async move {
        async fn calls() -> Result<()> {
            let client = get_test_client().await?;

            let result: Option<(String, String)> = client.brpop("list", 0.0).await?;
            assert_eq!(Some(("list".to_owned(), "element2".to_owned())), result);

            Ok(())
        }

        let _result = calls().await;
    });

    client.lpush("list", "element2").await?;

    sleep(Duration::from_millis(100)).await;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lindex() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let element: String = client.lindex("mylist", 0).await?;
    assert_eq!("element1", element);

    let element: String = client.lindex("mylist", -1).await?;
    assert_eq!("element3", element);

    let element: Value = client.lindex("mylist", 3).await?;
    assert!(matches!(element, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn linsert() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client.rpush("mylist", ["element1", "element3"]).await?;

    let result = client
        .linsert("mylist", LInsertWhere::After, "element1", "element2")
        .await?;
    assert_eq!(3, result);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(3, elements.len());
    assert_eq!("element1".to_string(), elements[0]);
    assert_eq!("element2".to_string(), elements[1]);
    assert_eq!("element3".to_string(), elements[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn llen() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let len = client.llen("mylist").await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lmove() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["mylist", "myotherlist"]).await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let element: String = client.lmove("mylist", "myotherlist", Right, Left).await?;
    assert_eq!("element3", element);

    let element: String = client.lmove("mylist", "myotherlist", Left, Right).await?;
    assert_eq!("element1", element);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(1, elements.len());
    assert_eq!("element2".to_string(), elements[0]);

    let elements: Vec<String> = client.lrange("myotherlist", 0, -1).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3".to_string(), elements[0]);
    assert_eq!("element1".to_string(), elements[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lmpop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .lpush(
            "mylist",
            ["element1", "element2", "element3", "element4", "element5"],
        )
        .await?;

    let result: (String, Vec<String>) = client.lmpop("mylist", Left, 1).await?;
    assert_eq!("mylist", result.0);
    assert_eq!(1, result.1.len());
    assert_eq!("element5".to_string(), result.1[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .lpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let elements: Vec<String> = client.lpop("mylist", 2).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3", elements[0].as_str());
    assert_eq!("element2", elements[1].as_str());

    let elements: Vec<String> = client.lpop("mylist", 1).await?;
    assert_eq!(1, elements.len());
    assert_eq!("element1", elements[0].as_str());

    let elements: Vec<String> = client.lpop("mylist", 1).await?;
    assert_eq!(0, elements.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpos() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let pos = client.lpos("mylist", "element2", Some(1), Some(1)).await?;
    assert_eq!(None, pos);

    let pos = client.lpos("mylist", "element2", Some(1), Some(3)).await?;
    assert_eq!(Some(1), pos);

    let pos: Vec<usize> = client
        .lpos_with_count("mylist", "element2", 1, Some(1), Some(1))
        .await?;
    assert_eq!(0, pos.len());

    let pos: Vec<usize> = client
        .lpos_with_count("mylist", "element2", 1, Some(1), Some(3))
        .await?;
    assert_eq!(1, pos.len());
    assert_eq!(1, pos[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpush() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    let size = client.lpush("mylist", "element1").await?;
    assert_eq!(1, size);

    let size = client.lpush("mylist", ["element2", "element3"]).await?;
    assert_eq!(3, size);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpushx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    let len = client.lpushx("mylist", "element1").await?;
    assert_eq!(0, len);

    client.lpush("mylist", "element1").await?;
    let len = client.lpush("mylist", "element2").await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lrange() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(3, elements.len());
    assert_eq!("element1".to_string(), elements[0]);
    assert_eq!("element2".to_string(), elements[1]);
    assert_eq!("element3".to_string(), elements[2]);

    let elements: Vec<String> = client.lrange("mylist", -2, 1).await?;
    assert_eq!(1, elements.len());
    assert_eq!("element2".to_string(), elements[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lrem() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element1", "element3"])
        .await?;

    let len = client.lrem("mylist", 3, "element1").await?;
    assert_eq!(2, len);

    let len = client.lrem("mylist", -1, "element1").await?;
    assert_eq!(0, len);

    let len = client.lrem("mylist", 0, "element3").await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lset() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element1", "element3"])
        .await?;

    client.lset("mylist", 0, "element4").await?;
    client.lset("mylist", -2, "element5").await?;

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(3, elements.len());
    assert_eq!("element4".to_string(), elements[0]);
    assert_eq!("element5".to_string(), elements[1]);
    assert_eq!("element3".to_string(), elements[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ltrim() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    client.ltrim("mylist", 1, -1).await?;

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element2".to_string(), elements[0]);
    assert_eq!("element3".to_string(), elements[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rpop() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .await?;

    let elements: Vec<String> = client.rpop("mylist", 2).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3", elements[0].as_str());
    assert_eq!("element2", elements[1].as_str());

    let elements: Vec<String> = client.rpop("mylist", 1).await?;
    assert_eq!(1, elements.len());
    assert_eq!("element1", elements[0].as_str());

    let elements: Vec<String> = client.rpop("mylist", 1).await?;
    assert_eq!(0, elements.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rpush() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").await?;

    let len = client.rpush("mylist", "element1").await?;
    assert_eq!(1, len);

    let len = client.rpush("mylist", ["element2", "element3"]).await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rpushx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["mylist", "myotherlist"]).await?;

    client.rpush("mylist", "element1").await?;

    let len = client.rpushx("mylist", "element2").await?;
    assert_eq!(2, len);

    let len = client.rpushx("myotherlist", "element2").await?;
    assert_eq!(0, len);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element1".to_string(), elements[0]);
    assert_eq!("element2".to_string(), elements[1]);

    let elements: Vec<String> = client.lrange("myotherlist", 0, -1).await?;
    assert_eq!(0, elements.len());

    Ok(())
}
