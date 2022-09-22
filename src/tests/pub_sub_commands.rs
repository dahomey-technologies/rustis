use crate::{
    resp::BulkString, tests::get_default_addr, Connection, ConnectionCommandResult, Error,
    FlushingMode, PubSubCommands, Result, ServerCommands, StringCommands,
};
use futures::StreamExt;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pubsub() -> Result<()> {
    let pub_sub = Connection::connect(get_default_addr()).await?;
    let connection = Connection::connect(get_default_addr()).await?;

    // cleanup
    connection.flushdb(FlushingMode::Sync).send().await?;

    let mut pub_sub_stream = pub_sub.subscribe("mychannel").await?;
    connection.publish("mychannel", "mymessage").send().await?;

    let value = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))?;
    assert!(matches!(value, Ok(BulkString::Binary(b)) if b.as_slice() == b"mymessage"));

    connection.set("key", "value").send().await?;
    let value: String = connection.get("key").send().await?;
    assert_eq!("value".to_string(), value);

    std::mem::drop(pub_sub_stream);

    let mut pub_sub_stream = pub_sub.subscribe("mychannel2").await?;
    connection.publish("mychannel2", "mymessage2").send().await?;

    let value = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))?;
    assert!(matches!(value, Ok(BulkString::Binary(b)) if b.as_slice() == b"mymessage2"));

    Ok(())
}
