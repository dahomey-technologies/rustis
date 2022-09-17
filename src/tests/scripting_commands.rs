use crate::{
    resp::BulkString, spawn, tests::get_default_addr, CallBuilder, ConnectionMultiplexer,
    FlushingMode, LibraryInfo, Result, ScriptingCommands, ServerCommands, StringCommands, NONE_ARG,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn eval() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let result: String = database
        .eval(CallBuilder::script("return ARGV[1]").args("hello"))
        .await?;
    assert_eq!("hello", result);

    database.set("key", "hello").await?;
    let result: String = database
        .eval(CallBuilder::script("return redis.call('GET', KEYS[1])").keys("key"))
        .await?;
    assert_eq!("hello", result);

    database.set("key", "hello").await?;
    let result: String = database
        .eval(
            CallBuilder::script("return redis.call('GET', KEYS[1])..\" \"..ARGV[1]..\"!\"")
                .keys("key")
                .args("world"),
        )
        .await?;
    assert_eq!("hello world!", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn evalsha() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let sha1: String = database.script_load("return ARGV[1]").await?;

    let result: String = database
        .evalsha(CallBuilder::sha1(sha1).args("hello"))
        .await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn fcall() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let library: String = database.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = database
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_delete() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let library: String = database.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = database
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    database.function_delete("mylib").await?;

    let result: Result<String> = database
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_dump() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.flushdb(FlushingMode::Sync).await?;

    let library: String = database.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = database
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    let serialized_payload: BulkString = database.function_dump().await?;
    assert!(serialized_payload.len() > 0);

    database.function_delete("mylib").await?;

    database
        .function_restore(serialized_payload, Default::default())
        .await?;

    let result: String = database
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_flush() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let library: String = database.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    database.function_flush(FlushingMode::Sync).await?;

    let list: Vec<LibraryInfo> = database.function_list(NONE_ARG, false).await?;
    assert_eq!(0, list.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_list() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    database.function_flush(FlushingMode::Sync).await?;

    let code = "#!lua name=mylib \n redis.register_function{function_name='myfunc', callback=function(keys, args) return args[1] end, flags={ 'no-writes' }, description='My description'}";
    let library: String = database.function_load(true, code).await?;
    assert_eq!("mylib", library);

    let libs: Vec<LibraryInfo> = database.function_list(NONE_ARG, false).await?;
    assert_eq!(1, libs.len());
    assert_eq!("mylib", libs[0].library_name);
    assert_eq!("LUA", libs[0].engine);
    assert_eq!(1, libs[0].functions.len());
    assert_eq!("myfunc", libs[0].functions[0].name);
    assert_eq!("My description", libs[0].functions[0].description);
    assert_eq!(1, libs[0].functions[0].flags.len());
    assert_eq!("no-writes", libs[0].functions[0].flags[0]);
    assert_eq!(None, libs[0].library_code);

    let libs: Vec<LibraryInfo> = database.function_list(NONE_ARG, true).await?;
    assert_eq!(1, libs.len());
    assert_eq!("mylib", libs[0].library_name);
    assert_eq!("LUA", libs[0].engine);
    assert_eq!(1, libs[0].functions.len());
    assert_eq!("myfunc", libs[0].functions[0].name);
    assert_eq!("My description", libs[0].functions[0].description);
    assert_eq!(1, libs[0].functions[0].flags.len());
    assert_eq!("no-writes", libs[0].functions[0].flags[0]);
    assert_eq!(Some(code.to_owned()), libs[0].library_code);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_stats() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let _ = database.function_kill().await;

    database.function_flush(FlushingMode::Sync).await?;

    let code = "#!lua name=mylib \n redis.register_function{function_name='myfunc', callback=function(keys, args) while (true) do end return args[1] end, flags={ 'no-writes' }, description='My description'}";
    let library: String = database.function_load(true, code).await?;
    assert_eq!("mylib", library);

    spawn(async move {
        async fn blocking_fcall() -> Result<()> {
            let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
            let database = connection.get_default_database();

            let _ = database
                .fcall::<String>(CallBuilder::function("myfunc").args("hello"))
                .await?;

            Ok(())
        }

        let _ = blocking_fcall().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let function_stat = database.function_stat().await?;
    assert!(function_stat.running_script.is_some());
    if let Some(running_script) = function_stat.running_script {
        assert_eq!("myfunc", running_script.name);
        assert_eq!(4, running_script.command.len());
        assert_eq!("FCALL", running_script.command[0]);
        assert_eq!("myfunc", running_script.command[1]);
        assert_eq!("0", running_script.command[2]);
        assert_eq!("hello", running_script.command[3]);
        assert!(running_script.duration_ms > 100);
    }
    assert!(function_stat.engines.contains_key("LUA"));
    assert_eq!(1, function_stat.engines["LUA"].libraries_count);
    assert_eq!(1, function_stat.engines["LUA"].functions_count);

    database.function_kill().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn script_exists() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let sha11: String = database.script_load("return ARGV[1]").await?;
    let sha12: String = database
        .script_load("return redis.call('GET', KEYS[1])")
        .await?;

    let result = database
        .script_exists([sha11, sha12, "unknwon".to_owned()])
        .await?;
    assert_eq!([true, true, false], &result[..]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn script_flush() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let sha11: String = database.script_load("return ARGV[1]").await?;
    let sha12: String = database
        .script_load("return redis.call('GET', KEYS[1])")
        .await?;

    database.script_flush(FlushingMode::Sync).await?;

    let result = database.script_exists([sha11, sha12]).await?;
    assert_eq!([false, false], &result[..]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn script_kill() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    let _ = database.script_kill().await;

    let sha1: String = database
        .script_load("while (true) do end return ARGV[1]")
        .await?;

    spawn(async move {
        async fn blocking_script(sha1: String) -> Result<()> {
            let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
            let database = connection.get_default_database();

            let _ = database
                .evalsha::<String>(CallBuilder::sha1(sha1).args("hello"))
                .await?;

            Ok(())
        }

        let _ = blocking_script(sha1).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    database.script_kill().await?;

    Ok(())
}
