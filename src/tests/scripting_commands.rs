use crate::{
    tests::get_default_addr, ConnectionMultiplexer, Result, ScriptingCommands, StringCommands,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn eval() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let result = database
        .eval("return ARGV[1]", None as Option<String>, Some("hello"))
        .await?;
    let value: String = result.into()?;
    assert_eq!("hello", value);

    database.set("key", "hello").await?;
    let result = database
        .eval(
            "return redis.call('GET', KEYS[1])",
            Some("key"),
            None as Option<String>,
        )
        .await?;
    let value: String = result.into()?;
    assert_eq!("hello", value);

    database.set("key", "hello").await?;
    let result = database
        .eval(
            "return redis.call('GET', KEYS[1])..\" \"..ARGV[1]..\"!\"",
            Some("key"),
            Some("world"),
        )
        .await?;
    let value: String = result.into()?;
    assert_eq!("hello world!", value);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn evalsha() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let sha1: String = database.script_load("return ARGV[1]").await?;

    let result = database
        .evalsha(sha1, None as Option<String>, Some("hello"))
        .await?;
    let value: String = result.into()?;
    assert_eq!("hello", value);

    Ok(())
}
