use crate::{
    resp::BulkString, tests::get_default_addr, ConnectionMultiplexer, Error, GenericCommands,
    PubSubCommands, Result, StringCommands,
};
use futures::StreamExt;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pubsub() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();
    let pub_sub = connection.get_pub_sub();

    // cleanup
    database.del("key").await?;

    let mut pub_sub_stream = pub_sub.subscribe("mychannel").await?;
    pub_sub.publish("mychannel", "mymessage").await?;

    let value = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))?;
    assert!(matches!(value, Ok(BulkString::Binary(b)) if b.as_slice() == b"mymessage"));

    database.set("key", "value").await?;
    let value: String = database.get("key").await?;
    assert_eq!("value".to_string(), value);

    std::mem::drop(pub_sub_stream);

    let mut pub_sub_stream = pub_sub.subscribe("mychannel2").await?;
    pub_sub.publish("mychannel2", "mymessage2").await?;

    let value = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))?;
    assert!(matches!(value, Ok(BulkString::Binary(b)) if b.as_slice() == b"mymessage2"));

    Ok(())
}
