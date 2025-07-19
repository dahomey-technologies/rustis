use crate::{
    Result,
    client::ClientPreparedCommand,
    commands::{
        CallBuilder, FlushingMode, FunctionListOptions, LibraryInfo, ScriptingCommands,
        ServerCommands, StringCommands,
    },
    error::{Error, RedisErrorKind},
    sleep, spawn,
    tests::get_test_client,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn eval() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client
        .eval(CallBuilder::script("return ARGV[1]").args("hello"))
        .await?;
    assert_eq!("hello", result);

    client.set("key", "hello").await?;
    let result: String = client
        .eval(CallBuilder::script("return redis.call('GET', KEYS[1])").keys("key"))
        .await?;
    assert_eq!("hello", result);

    client.set("key", "hello").await?;
    let result: String = client
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
async fn eval_tuple_response() -> Result<()> {
    let client = get_test_client().await?;

    let lua_script = r#"
redis.call("DEL", "key");
redis.call("SADD", "key", 1, 2, 3, 4);
local arr = redis.call("SMEMBERS", "key");
redis.call("DEL", "key");
return { ARGV[1], ARGV[2], 42, arr }
    "#;
    let result: (String, String, i32, Vec<i64>) = client
        .eval(CallBuilder::script(lua_script).args("Hello").args("world"))
        .await?;

    assert_eq!(result.0, "Hello");
    assert_eq!(result.1, "world");
    assert_eq!(result.2, 42);
    assert_eq!(result.3, vec![1, 2, 3, 4]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn evalsha_noscript() -> Result<()> {
    let client = get_test_client().await?;

    // SHA1("") == da39a3ee5e6b4b0d3255bfef95601890afd80709
    let result = client
        .evalsha::<()>(CallBuilder::sha1(
            "da39a3ee5e6b4b0d3255bfef95601890afd80709",
        ))
        .await
        .unwrap_err();

    let Error::Redis(error) = result else {
        unreachable!();
    };

    assert_eq!(error.kind, RedisErrorKind::NoScript);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn evalsha() -> Result<()> {
    let client = get_test_client().await?;

    let sha1: String = client.script_load("return ARGV[1]").await?;

    let result: String = client
        .evalsha(CallBuilder::sha1(sha1).args("hello"))
        .await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn fcall() -> Result<()> {
    let client = get_test_client().await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = client
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn fcall_tuple_response() -> Result<()> {
    let client = get_test_client().await?;

    let lua_lib = r#"#!lua name=mylib
redis.register_function('myfunc', function(keys, args) 
    redis.call("DEL", "key");
    redis.call("SADD", "key", 1, 2, 3, 4);
    local arr = redis.call("SMEMBERS", "key");
    redis.call("DEL", "key");
    return { args[1], args[2], 42, arr }
end)
    "#;
    let library: String = client.function_load(true, lua_lib).await?;
    assert_eq!("mylib", library);
    let result: (String, String, i32, Vec<i64>) = client
        .fcall(CallBuilder::function("myfunc").args("Hello").args("world"))
        .await?;

    assert_eq!(result.0, "Hello");
    assert_eq!(result.1, "world");
    assert_eq!(result.2, 42);
    assert_eq!(result.3, vec![1, 2, 3, 4]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_delete() -> Result<()> {
    let client = get_test_client().await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = client
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    client.function_delete("mylib").await?;

    let result: Result<String> = client
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await;
    assert!(result.is_err());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_dump() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = client
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    let serialized_payload = client.function_dump().await?;
    assert!(!serialized_payload.0.is_empty());

    client.function_delete("mylib").await?;

    client
        .function_restore(serialized_payload.0, Default::default())
        .await?;

    let result: String = client
        .fcall(CallBuilder::function("myfunc").args("hello"))
        .await?;
    assert_eq!("hello", result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_flush() -> Result<()> {
    let client = get_test_client().await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    client.function_flush(FlushingMode::Sync).await?;

    let list: Vec<LibraryInfo> = client.function_list(FunctionListOptions::default()).await?;
    assert_eq!(0, list.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn function_list() -> Result<()> {
    let client = get_test_client().await?;

    client.function_flush(FlushingMode::Sync).await?;

    let code = "#!lua name=mylib \n redis.register_function{function_name='myfunc', callback=function(keys, args) return args[1] end, flags={ 'no-writes' }, description='My description'}";
    let library: String = client.function_load(true, code).await?;
    assert_eq!("mylib", library);

    let libs: Vec<LibraryInfo> = client.function_list(FunctionListOptions::default()).await?;
    assert_eq!(1, libs.len());
    assert_eq!("mylib", libs[0].library_name);
    assert_eq!("LUA", libs[0].engine);
    assert_eq!(1, libs[0].functions.len());
    assert_eq!("myfunc", libs[0].functions[0].name);
    assert_eq!("My description", libs[0].functions[0].description);
    assert_eq!(1, libs[0].functions[0].flags.len());
    assert_eq!("no-writes", libs[0].functions[0].flags[0]);
    assert_eq!(None, libs[0].library_code);

    let libs: Vec<LibraryInfo> = client
        .function_list(FunctionListOptions::default().with_code())
        .await?;
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
    let client = get_test_client().await?;

    client.function_kill().forget()?;

    client.function_flush(FlushingMode::Sync).await?;

    let code = "#!lua name=mylib \n redis.register_function{function_name='myfunc', callback=function(keys, args) while (true) do end return args[1] end, flags={ 'no-writes' }, description='My description'}";
    let library: String = client.function_load(true, code).await?;
    assert_eq!("mylib", library);

    spawn(async move {
        async fn blocking_fcall() -> Result<()> {
            let client = get_test_client().await?;

            let _ = client
                .fcall::<String>(CallBuilder::function("myfunc").args("hello"))
                .await?;

            Ok(())
        }

        let _ = blocking_fcall().await;
    });

    sleep(std::time::Duration::from_millis(100)).await;

    let function_stat = client.function_stats().await?;
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

    client.function_kill().await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn script_exists() -> Result<()> {
    let client = get_test_client().await?;

    let sha11: String = client.script_load("return ARGV[1]").await?;
    let sha12: String = client
        .script_load("return redis.call('GET', KEYS[1])")
        .await?;

    let result = client
        .script_exists([sha11, sha12, "unknwon".to_owned()])
        .await?;
    assert_eq!([true, true, false], &result[..]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn script_flush() -> Result<()> {
    let client = get_test_client().await?;

    let sha11: String = client.script_load("return ARGV[1]").await?;
    let sha12: String = client
        .script_load("return redis.call('GET', KEYS[1])")
        .await?;

    client.script_flush(FlushingMode::Sync).await?;

    let result = client.script_exists([sha11, sha12]).await?;
    assert_eq!([false, false], &result[..]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn script_kill() -> Result<()> {
    let client = get_test_client().await?;

    let _ = client.script_kill().await;

    let sha1: String = client
        .script_load("while (true) do end return ARGV[1]")
        .await?;

    spawn(async move {
        async fn blocking_script(sha1: String) -> Result<()> {
            let client = get_test_client().await?;

            let _ = client
                .evalsha::<String>(CallBuilder::sha1(sha1).args("hello"))
                .await?;

            Ok(())
        }

        let _ = blocking_script(sha1).await;
    });

    sleep(std::time::Duration::from_millis(100)).await;

    client.script_kill().await?;

    Ok(())
}
