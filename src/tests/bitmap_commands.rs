use crate::{
    resp::BulkString, tests::get_default_addr, BitFieldGetSubCommand, BitFieldOverflow,
    BitFieldSubCommand, BitOperation, BitRange, BitUnit, BitmapCommands, ConnectionMultiplexer,
    DatabaseCommandResult, Result, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitcount() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("mykey", "foobar").send().await?;

    let count = database.bitcount("mykey", BitRange::default()).send().await?;
    assert_eq!(26, count);

    let count = database
        .bitcount("mykey", BitRange::range(0, 0))
        .send()
        .await?;
    assert_eq!(4, count);

    let count = database
        .bitcount("mykey", BitRange::range(1, 1))
        .send()
        .await?;
    assert_eq!(6, count);

    let count = database
        .bitcount("mykey", BitRange::range(1, 1).unit(BitUnit::Byte))
        .send()
        .await?;
    assert_eq!(6, count);

    let count = database
        .bitcount("mykey", BitRange::range(5, 30).unit(BitUnit::Bit))
        .send()
        .await?;
    assert_eq!(17, count);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitfield() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("mykey", "foobar").send().await?;

    let results = database
        .bitfield(
            "mykey",
            [
                BitFieldSubCommand::incr_by("i5", 100, 1),
                BitFieldSubCommand::get("u4", 0),
            ],
        )
        .send()
        .await?;
    assert!(matches!(results[..], [1, 6]));

    database.set("mykey", "foobar").send().await?;

    let results = database
        .bitfield(
            "mykey",
            [
                BitFieldSubCommand::set("i8", "#0", 65),
                BitFieldSubCommand::set("i8", "#1", 66),
            ],
        )
        .send()
        .await?;
    assert!(matches!(results[..], [102, 111]));

    database.set("mykey", "foobar").send().await?;

    let results = database
        .bitfield(
            "mykey",
            [
                BitFieldSubCommand::incr_by("u2", "100", 1),
                BitFieldSubCommand::overflow(BitFieldOverflow::Sat),
                BitFieldSubCommand::incr_by("u2", "102", 1),
            ],
        )
        .send()
        .await?;
    assert!(matches!(results[..], [1, 1]));

    let results = database
        .bitfield(
            "mykey",
            [BitFieldSubCommand::<String, String>::overflow(
                BitFieldOverflow::Fail,
            )],
        )
        .send()
        .await?;
    assert_eq!(0, results.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitfield_readonly() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("mykey", "foobar").send().await?;

    let results = database
        .bitfield_readonly("mykey", [BitFieldGetSubCommand::new("i8", 0)])
        .send()
        .await?;
    assert_eq!(1, results.len());
    assert_eq!(b'f' as u64, results[0]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitop() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("key1", "foobar").send().await?;
    database.set("key2", "abcdef").send().await?;

    let len = database
        .bitop(BitOperation::And, "dest", ["key1", "key2"])
        .send()
        .await?;
    assert_eq!(6, len);

    let value: String = database.get("dest").send().await?;
    assert_eq!("`bc`ab", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn bitpos() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database
        .set("mykey", BulkString::Binary(vec![0xFFu8, 0xF0u8, 0x00u8]))
        .send()
        .await?;

    let pos = database.bitpos("mykey", 1, BitRange::default()).send().await?;
    assert_eq!(0, pos);

    database
        .set("mykey", BulkString::Binary(vec![0x00u8, 0xFFu8, 0xF0u8]))
        .send()
        .await?;
    let pos = database
        .bitpos("mykey", 0, BitRange::range(0, -1))
        .send()
        .await?;
    assert_eq!(0, pos);

    let pos = database
        .bitpos("mykey", 1, BitRange::range(2, -1))
        .send()
        .await?;
    assert_eq!(16, pos);

    let pos = database
        .bitpos("mykey", 1, BitRange::range(2, -1).unit(BitUnit::Byte))
        .send()
        .await?;
    assert_eq!(16, pos);

    let pos = database
        .bitpos("mykey", 1, BitRange::range(7, 15).unit(BitUnit::Bit))
        .send()
        .await?;
    assert_eq!(8, pos);

    let pos = database
        .bitpos("mykey", 1, BitRange::range(7, -3).unit(BitUnit::Bit))
        .send()
        .await?;
    assert_eq!(8, pos);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn getbit() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("mykey", "foobar").send().await?;

    let value = database.getbit("mykey", 6).send().await?;
    assert_eq!(1, value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn setbit() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.set("mykey", "foobar").send().await?;

    let value = database.setbit("mykey", 7, 1).send().await?;
    assert_eq!(0, value);

    let value = database.setbit("mykey", 7, 0).send().await?;
    assert_eq!(1, value);

    let value = database.getbit("mykey", 7).send().await?;
    assert_eq!(0, value);

    Ok(())
}
