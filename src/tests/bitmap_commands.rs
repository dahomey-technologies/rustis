use crate::{
    resp::BulkString, tests::get_test_client, BitFieldGetSubCommand, BitFieldOverflow,
    BitFieldSubCommand, BitOperation, BitRange, BitUnit, BitmapCommands,
    Result, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitcount() -> Result<()> {
    let client = get_test_client().await?;

    client.set("mykey", "foobar").await?;

    let count = client.bitcount("mykey", BitRange::default()).await?;
    assert_eq!(26, count);

    let count = client.bitcount("mykey", BitRange::range(0, 0)).await?;
    assert_eq!(4, count);

    let count = client.bitcount("mykey", BitRange::range(1, 1)).await?;
    assert_eq!(6, count);

    let count = client
        .bitcount("mykey", BitRange::range(1, 1).unit(BitUnit::Byte))
        .await?;
    assert_eq!(6, count);

    let count = client
        .bitcount("mykey", BitRange::range(5, 30).unit(BitUnit::Bit))
        .await?;
    assert_eq!(17, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitfield() -> Result<()> {
    let client = get_test_client().await?;

    client.set("mykey", "foobar").await?;

    let results = client
        .bitfield(
            "mykey",
            [
                BitFieldSubCommand::incr_by("i5", 100, 1),
                BitFieldSubCommand::get("u4", 0),
            ],
        )
        .await?;
    assert!(matches!(results[..], [1, 6]));

    client.set("mykey", "foobar").await?;

    let results = client
        .bitfield(
            "mykey",
            [
                BitFieldSubCommand::set("i8", "#0", 65),
                BitFieldSubCommand::set("i8", "#1", 66),
            ],
        )
        .await?;
    assert!(matches!(results[..], [102, 111]));

    client.set("mykey", "foobar").await?;

    let results = client
        .bitfield(
            "mykey",
            [
                BitFieldSubCommand::incr_by("u2", "100", 1),
                BitFieldSubCommand::overflow(BitFieldOverflow::Sat),
                BitFieldSubCommand::incr_by("u2", "102", 1),
            ],
        )
        .await?;
    assert!(matches!(results[..], [1, 1]));

    let results = client
        .bitfield(
            "mykey",
            [BitFieldSubCommand::<String, String>::overflow(
                BitFieldOverflow::Fail,
            )],
        )
        .await?;
    assert_eq!(0, results.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitfield_readonly() -> Result<()> {
    let client = get_test_client().await?;

    client.set("mykey", "foobar").await?;

    let results = client
        .bitfield_readonly("mykey", [BitFieldGetSubCommand::new("i8", 0)])
        .await?;
    assert_eq!(1, results.len());
    assert_eq!(b'f' as u64, results[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitop() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key1", "foobar").await?;
    client.set("key2", "abcdef").await?;

    let len = client
        .bitop(BitOperation::And, "dest", ["key1", "key2"])
        .await?;
    assert_eq!(6, len);

    let value: String = client.get("dest").await?;
    assert_eq!("`bc`ab", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitpos() -> Result<()> {
    let client = get_test_client().await?;

    client
        .set("mykey", BulkString::Binary(vec![0xFFu8, 0xF0u8, 0x00u8]))
        .await?;

    let pos = client.bitpos("mykey", 1, BitRange::default()).await?;
    assert_eq!(0, pos);

    client
        .set("mykey", BulkString::Binary(vec![0x00u8, 0xFFu8, 0xF0u8]))
        .await?;
    let pos = client.bitpos("mykey", 0, BitRange::range(0, -1)).await?;
    assert_eq!(0, pos);

    let pos = client.bitpos("mykey", 1, BitRange::range(2, -1)).await?;
    assert_eq!(16, pos);

    let pos = client
        .bitpos("mykey", 1, BitRange::range(2, -1).unit(BitUnit::Byte))
        .await?;
    assert_eq!(16, pos);

    let pos = client
        .bitpos("mykey", 1, BitRange::range(7, 15).unit(BitUnit::Bit))
        .await?;
    assert_eq!(8, pos);

    let pos = client
        .bitpos("mykey", 1, BitRange::range(7, -3).unit(BitUnit::Bit))
        .await?;
    assert_eq!(8, pos);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getbit() -> Result<()> {
    let client = get_test_client().await?;

    client.set("mykey", "foobar").await?;

    let value = client.getbit("mykey", 6).await?;
    assert_eq!(1, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setbit() -> Result<()> {
    let client = get_test_client().await?;

    client.set("mykey", "foobar").await?;

    let value = client.setbit("mykey", 7, 1).await?;
    assert_eq!(0, value);

    let value = client.setbit("mykey", 7, 0).await?;
    assert_eq!(1, value);

    let value = client.getbit("mykey", 7).await?;
    assert_eq!(0, value);

    Ok(())
}
