use std::collections::{HashSet, HashMap};

use crate::{
    tests::get_test_client, Error, FlushingMode, PubSubChannelsOptions, PubSubCommands, Result,
    ServerCommands, StringCommands,
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
        .ok_or_else(|| Error::Client("fail".to_owned()))??
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
        .ok_or_else(|| Error::Client("fail".to_owned()))??
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
        .ok_or_else(|| Error::Client("fail".to_owned()))??
        .into()?;

    assert_eq!("mychannel1", channel);
    assert_eq!("mymessage1", message);

    let (channel, message): (String, String) = pub_sub_stream
        .next()
        .await
        .ok_or_else(|| Error::Client("fail".to_owned()))??
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
        .ok_or_else(|| Error::Client("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel11", channel);
    assert_eq!("mymessage11", message);

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or_else(|| Error::Client("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel1*", pattern);
    assert_eq!("mychannel12", channel);
    assert_eq!("mymessage12", message);

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or_else(|| Error::Client("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel21", channel);
    assert_eq!("mymessage21", message);

    let (pattern, channel, message): (String, String, String) = pub_sub_stream
        .next()
        .await
        .ok_or_else(|| Error::Client("fail".to_owned()))??
        .into()?;
    assert_eq!("mychannel2*", pattern);
    assert_eq!("mychannel22", channel);
    assert_eq!("mymessage22", message.to_string());

    pub_sub_stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_channels() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let mut stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2", "mychannel3", "otherchannel"])
        .await?;

    let channels: HashSet<String> = regular_client.pub_sub_channels(Default::default()).await?;
    assert_eq!(4, channels.len());
    assert!(channels.contains("mychannel1"));
    assert!(channels.contains("mychannel2"));
    assert!(channels.contains("mychannel3"));
    assert!(channels.contains("otherchannel"));

    let channels: HashSet<String> = regular_client
        .pub_sub_channels(PubSubChannelsOptions::default().pattern("mychannel*"))
        .await?;
    assert_eq!(3, channels.len());
    assert!(channels.contains("mychannel1"));
    assert!(channels.contains("mychannel2"));
    assert!(channels.contains("mychannel3"));

    stream.close().await?;

    let channels: HashSet<String> = regular_client.pub_sub_channels(Default::default()).await?;
    assert_eq!(0, channels.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_numpat() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(0, num_patterns);

    let mut stream = pub_sub_client.psubscribe(["mychannel*"]).await?;

    let num_patterns = regular_client.pub_sub_numpat().await?;
    assert_eq!(1, num_patterns);

    stream.close().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn pub_sub_numsub() -> Result<()> {
    let pub_sub_client = get_test_client().await?;
    let regular_client = get_test_client().await?;

    let num_sub: Vec<(String, usize)> = regular_client
        .pub_sub_numsub(["mychannel1", "mychannel2"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(("mychannel1".to_string(), 0), num_sub[0]);
    assert_eq!(("mychannel2".to_string(), 0), num_sub[1]);

    let mut stream = pub_sub_client
        .subscribe(["mychannel1", "mychannel2"])
        .await?;

    let num_sub: HashMap<String, usize> = regular_client
        .pub_sub_numsub(["mychannel1", "mychannel2"])
        .await?;
    assert_eq!(2, num_sub.len());
    assert_eq!(Some(&1usize), num_sub.get("mychannel1"));
    assert_eq!(Some(&1usize), num_sub.get("mychannel2"));

    stream.close().await?;

    Ok(())
}
