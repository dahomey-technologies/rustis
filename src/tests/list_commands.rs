use crate::{
    resp::{BulkString, Value},
    tests::get_test_client,
    ConnectionCommandResult, GenericCommands, LInsertWhere,
    LMoveWhere::Left,
    LMoveWhere::Right,
    ListCommands, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lindex() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let element: String = client.lindex("mylist", 0).send().await?;
    assert_eq!("element1", element);

    let element: String = client.lindex("mylist", -1).send().await?;
    assert_eq!("element3", element);

    let element: Value = client.lindex("mylist", 3).send().await?;
    assert!(matches!(element, Value::BulkString(BulkString::Nil)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn linsert() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element3"])
        .send()
        .await?;

    let result = client
        .linsert("mylist", LInsertWhere::After, "element1", "element2")
        .send()
        .await?;
    assert_eq!(3, result);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).send().await?;
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
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let len = client.llen("mylist").send().await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lmove() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["mylist", "myotherlist"]).send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let element: String = client
        .lmove("mylist", "myotherlist", Right, Left)
        .send()
        .await?;
    assert_eq!("element3", element);

    let element: String = client
        .lmove("mylist", "myotherlist", Left, Right)
        .send()
        .await?;
    assert_eq!("element1", element);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).send().await?;
    assert_eq!(1, elements.len());
    assert_eq!("element2".to_string(), elements[0]);

    let elements: Vec<String> = client.lrange("myotherlist", 0, -1).send().await?;
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
    client.del("mylist").send().await?;

    client
        .lpush(
            "mylist",
            ["element1", "element2", "element3", "element4", "element5"],
        )
        .send()
        .await?;

    let result: (String, Vec<String>) = client.lmpop("mylist", Left, 1).send().await?;
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
    client.del("mylist").send().await?;

    client
        .lpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let elements: Vec<String> = client.lpop("mylist", 2).send().await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3", elements[0].as_str());
    assert_eq!("element2", elements[1].as_str());

    let elements: Vec<String> = client.lpop("mylist", 1).send().await?;
    assert_eq!(1, elements.len());
    assert_eq!("element1", elements[0].as_str());

    let elements: Vec<String> = client.lpop("mylist", 1).send().await?;
    assert_eq!(0, elements.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpos() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let pos = client
        .lpos("mylist", "element2", Some(1), Some(1))
        .send()
        .await?;
    assert_eq!(None, pos);

    let pos = client
        .lpos("mylist", "element2", Some(1), Some(3))
        .send()
        .await?;
    assert_eq!(Some(1), pos);

    let pos: Vec<usize> = client
        .lpos_with_count("mylist", "element2", 1, Some(1), Some(1))
        .send()
        .await?;
    assert_eq!(0, pos.len());

    let pos: Vec<usize> = client
        .lpos_with_count("mylist", "element2", 1, Some(1), Some(3))
        .send()
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
    client.del("mylist").send().await?;

    let size = client.lpush("mylist", "element1").send().await?;
    assert_eq!(1, size);

    let size = client
        .lpush("mylist", ["element2", "element3"])
        .send()
        .await?;
    assert_eq!(3, size);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpushx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    let len = client.lpushx("mylist", "element1").send().await?;
    assert_eq!(0, len);

    client.lpush("mylist", "element1").send().await?;
    let len = client.lpush("mylist", "element2").send().await?;
    assert_eq!(2, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lrange() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let elements: Vec<String> = client.lrange("mylist", 0, -1).send().await?;
    assert_eq!(3, elements.len());
    assert_eq!("element1".to_string(), elements[0]);
    assert_eq!("element2".to_string(), elements[1]);
    assert_eq!("element3".to_string(), elements[2]);

    let elements: Vec<String> = client.lrange("mylist", -2, 1).send().await?;
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
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element1", "element3"])
        .send()
        .await?;

    let len = client.lrem("mylist", 3, "element1").send().await?;
    assert_eq!(2, len);

    let len = client.lrem("mylist", -1, "element1").send().await?;
    assert_eq!(0, len);

    let len = client.lrem("mylist", 0, "element3").send().await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lset() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element1", "element3"])
        .send()
        .await?;

    client.lset("mylist", 0, "element4").send().await?;
    client.lset("mylist", -2, "element5").send().await?;

    let elements: Vec<String> = client.lrange("mylist", 0, -1).send().await?;
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
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    client.ltrim("mylist", 1, -1).send().await?;

    let elements: Vec<String> = client.lrange("mylist", 0, -1).send().await?;
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
    client.del("mylist").send().await?;

    client
        .rpush("mylist", ["element1", "element2", "element3"])
        .send()
        .await?;

    let elements: Vec<String> = client.rpop("mylist", 2).send().await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3", elements[0].as_str());
    assert_eq!("element2", elements[1].as_str());

    let elements: Vec<String> = client.rpop("mylist", 1).send().await?;
    assert_eq!(1, elements.len());
    assert_eq!("element1", elements[0].as_str());

    let elements: Vec<String> = client.rpop("mylist", 1).send().await?;
    assert_eq!(0, elements.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rpush() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("mylist").send().await?;

    let len = client.rpush("mylist", "element1").send().await?;
    assert_eq!(1, len);

    let len = client
        .rpush("mylist", ["element2", "element3"])
        .send()
        .await?;
    assert_eq!(3, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn rpushx() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["mylist", "myotherlist"]).send().await?;

    client.rpush("mylist", "element1").send().await?;

    let len = client.rpushx("mylist", "element2").send().await?;
    assert_eq!(2, len);

    let len = client.rpushx("myotherlist", "element2").send().await?;
    assert_eq!(0, len);

    let elements: Vec<String> = client.lrange("mylist", 0, -1).send().await?;
    assert_eq!(2, elements.len());
    assert_eq!("element1".to_string(), elements[0]);
    assert_eq!("element2".to_string(), elements[1]);

    let elements: Vec<String> = client.lrange("myotherlist", 0, -1).send().await?;
    assert_eq!(0, elements.len());

    Ok(())
}
