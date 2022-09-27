use crate::{
    tests::get_test_client, Error, FlushingMode, PubSubCommands, Result, ServerCommands,
    StringCommands,
};
use futures::StreamExt;
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pubsub() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel").await?;
    regular_client.publish("mychannel", "mymessage").await?;

    let (channel, message): (String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;

    assert_eq!("mychannel", channel);
    assert_eq!("mymessage", message);

    regular_client.set("key", "value").await?;
    let value: String = regular_client.get("key").await?;
    assert_eq!("value", value);

    pub_sub_stream.close().await?;

    let mut pub_sub_stream = pub_sub_client.subscribe("mychannel2").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let (channel, message): (String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", message);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn forbidden_command() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.flushdb(FlushingMode::Sync).await?;

    // regular mode, these commands are allowed
    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    // subscribed mode
    let mut pub_sub_stream = client.subscribe("mychannel").await?;

    // Cannot send regular commands during subscribed mode
    let result: Result<String> = client.get("key").await;
    assert!(result.is_err());

    pub_sub_stream.close().await?;

    // After leaving subscribed mode, should work again
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn subscribe_to_multiple_channels() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;
    regular_client.publish("mychannel1", "mymessage1").await?;
    regular_client.publish("mychannel2", "mymessage2").await?;

    let (channel, message): (String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", message);

    let (channel, message): (String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;

    assert_eq!("mychannel2", channel);
    assert_eq!("mymessage2", message);

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn subscribe_to_multiple_patterns() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    // cleanup
    regular_client.flushdb(FlushingMode::Sync).await?;

    let mut pub_sub_stream = pub_sub_client
        .psubscribe(["mychannel1*", "mychannel2*"])
        .await?;

    regular_client.publish("mychannel11", "mymessage11").await?;
    regular_client.publish("mychannel12", "mymessage12").await?;
    regular_client.publish("mychannel21", "mymessage21").await?;
    regular_client.publish("mychannel22", "mymessage22").await?;

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel11", channel);
    assert_eq!("mymessage11", message);

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel12", channel);
    assert_eq!("mymessage12", message);

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel21", channel);
    assert_eq!("mymessage21", message);

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or(Error::Internal("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel22", channel);
    assert_eq!("mymessage22", message.to_string());

    pub_sub_stream.close().await?;

    Ok(())
}
