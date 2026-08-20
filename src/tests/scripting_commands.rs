use crate::{
    ErrorKind, Result,
    client::ClientPreparedCommand,
    commands::{
        FlushingMode, FunctionListOptions, LibraryInfo, ScriptDebugMode, ScriptingCommands,
        ServerCommands, StringCommands,
    },
    error::RedisErrorKind,
    sleep, spawn,
    tests::get_test_client,
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn eval() -> Result<()> {
    let client = get_test_client().await?;

    let result: String = client.eval("return ARGV[1]", (), "hello").await?;
    assert_eq!("hello", result);

    client.set("key", "hello").await?;
    let result: String = client
        .eval("return redis.call('GET', KEYS[1])", "key", ())
        .await?;
    assert_eq!("hello", result);

    client.set("key", "hello").await?;
    let result: String = client
        .eval(
            "return redis.call('GET', KEYS[1])..\" \"..ARGV[1]..\"!\"",
            "key",
            "world",
        )
        .await?;
    assert_eq!("hello world!", result);

    Ok(())
}

#[tokio::test]
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
    let result: (String, String, i32, Vec<i64>) =
        client.eval(lua_script, (), ["Hello", "world"]).await?;

    assert_eq!(result.0, "Hello");
    assert_eq!(result.1, "world");
    assert_eq!(result.2, 42);
    assert_eq!(result.3, vec![1, 2, 3, 4]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn evalsha_noscript() -> Result<()> {
    let client = get_test_client().await?;

    // SHA1("") == da39a3ee5e6b4b0d3255bfef95601890afd80709
    let result = client
        .evalsha::<()>("da39a3ee5e6b4b0d3255bfef95601890afd80709", (), ())
        .await
        .unwrap_err();

    let ErrorKind::Redis(error) = result.into_kind() else {
        unreachable!();
    };

    assert_eq!(error.kind, RedisErrorKind::NoScript);

    Ok(())
}

#[tokio::test]
#[serial]
async fn evalsha() -> Result<()> {
    let client = get_test_client().await?;

    let sha1: String = client.script_load("return ARGV[1]").await?;

    let result: String = client.evalsha(sha1, (), "hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

#[tokio::test]
#[serial]
async fn fcall() -> Result<()> {
    let client = get_test_client().await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = client.fcall("myfunc", (), "hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

#[tokio::test]
#[serial]
async fn eval_readonly() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "hello").await?;

    let result: String = client
        .eval_readonly("return redis.call('GET', KEYS[1])", "key", ())
        .await?;
    assert_eq!("hello", result);

    let result: String = client
        .eval_readonly(
            "return redis.call('GET', KEYS[1])..\" \"..ARGV[1]..\"!\"",
            "key",
            "world",
        )
        .await?;
    assert_eq!("hello world!", result);

    // A write from a read-only script is rejected by the server.
    let result: Result<()> = client
        .eval_readonly("return redis.call('SET', KEYS[1], 'other')", "key", ())
        .await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn evalsha_readonly() -> Result<()> {
    let client = get_test_client().await?;

    client.set("key", "hello").await?;

    let sha1: String = client
        .script_load("return redis.call('GET', KEYS[1])")
        .await?;

    let result: String = client.evalsha_readonly(sha1.clone(), "key", ()).await?;
    assert_eq!("hello", result);

    let sha1: String = client
        .script_load("return redis.call('SET', KEYS[1], 'other')")
        .await?;

    let result: Result<()> = client.evalsha_readonly(sha1, "key", ()).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn fcall_readonly() -> Result<()> {
    let client = get_test_client().await?;

    client.function_flush(FlushingMode::Sync).await?;

    // Only a function registered with the `no-writes` flag is callable through
    // FCALL_RO.
    let library: String = client
        .function_load(
            true,
            "#!lua name=mylib \n redis.register_function{function_name='myfunc', callback=function(keys, args) return args[1] end, flags={'no-writes'}}",
        )
        .await?;
    assert_eq!("mylib", library);

    let result: String = client.fcall_readonly("myfunc", (), "hello").await?;
    assert_eq!("hello", result);

    let library: String = client
        .function_load(
            true,
            "#!lua name=writelib \n redis.register_function('writefunc', function(keys, args) return redis.call('SET', keys[1], args[1]) end)",
        )
        .await?;
    assert_eq!("writelib", library);

    let result: Result<()> = client.fcall_readonly("writefunc", "key", "value").await;
    assert!(result.is_err());

    client.function_flush(FlushingMode::Sync).await?;

    Ok(())
}

#[tokio::test]
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
    let result: (String, String, i32, Vec<i64>) =
        client.fcall("myfunc", (), ["Hello", "world"]).await?;

    assert_eq!(result.0, "Hello");
    assert_eq!(result.1, "world");
    assert_eq!(result.2, 42);
    assert_eq!(result.3, vec![1, 2, 3, 4]);

    Ok(())
}

#[tokio::test]
#[serial]
async fn function_delete() -> Result<()> {
    let client = get_test_client().await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = client.fcall("myfunc", (), "hello").await?;
    assert_eq!("hello", result);

    client.function_delete("mylib").await?;

    let result: Result<String> = client.fcall("myfunc", (), "hello").await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn function_dump() -> Result<()> {
    let client = get_test_client().await?;

    client.flushdb(FlushingMode::Sync).await?;

    let library: String = client.function_load(true, "#!lua name=mylib \n redis.register_function('myfunc', function(keys, args) return args[1] end)").await?;
    assert_eq!("mylib", library);

    let result: String = client.fcall("myfunc", (), "hello").await?;
    assert_eq!("hello", result);

    let serialized_payload = client.function_dump().await?;
    assert!(!serialized_payload.is_empty());

    client.function_delete("mylib").await?;

    client.function_restore(&serialized_payload, None).await?;

    let result: String = client.fcall("myfunc", (), "hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

#[tokio::test]
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

#[tokio::test]
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
    assert_eq!(
        Some("My description".to_owned()),
        libs[0].functions[0].description
    );
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
    assert_eq!(
        Some("My description".to_owned()),
        libs[0].functions[0].description
    );
    assert_eq!(1, libs[0].functions[0].flags.len());
    assert_eq!("no-writes", libs[0].functions[0].flags[0]);
    assert_eq!(Some(code.to_owned()), libs[0].library_code);

    Ok(())
}

#[tokio::test]
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

            let _ = client.fcall::<String>("myfunc", (), "hello").await?;

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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

            let _ = client.evalsha::<String>(sha1, (), "hello").await?;

            Ok(())
        }

        let _ = blocking_script(sha1).await;
    });

    sleep(std::time::Duration::from_millis(100)).await;

    client.script_kill().await?;

    Ok(())
}

/// `FUNCTION HELP` answers the subcommand list as a flat array of text lines,
/// which is the shape the declared return type claims.
#[tokio::test]
#[serial]
async fn function_help() -> Result<()> {
    let client = get_test_client().await?;

    let help = client.function_help().await?;

    assert!(help.iter().any(|line| line.contains("LOAD")));

    Ok(())
}

/// `SCRIPT DEBUG` takes one of three modes. `No` is the server default, so it
/// is the one mode a test can send without leaving the connection in a state
/// that stalls every script the rest of the suite runs.
#[tokio::test]
#[serial]
async fn script_debug() -> Result<()> {
    let client = get_test_client().await?;

    client.script_debug(ScriptDebugMode::No).await?;

    let result: String = client.eval("return ARGV[1]", (), "hello").await?;
    assert_eq!("hello", result);

    Ok(())
}

/// `FUNCTION LIST [LIBRARYNAME library-name] [WITHCODE]`. LIBRARYNAME is an exact
/// library name, so a name that exists selects one library and any other selects
/// none.
#[tokio::test]
#[serial]
async fn function_list_library_name_pattern() -> Result<()> {
    let client = get_test_client().await?;

    client.function_flush(FlushingMode::Sync).await?;

    let code = "#!lua name=mylib \n redis.register_function{function_name='myfunc', callback=function(keys, args) return args[1] end, flags={ 'no-writes' }}";
    let library: String = client.function_load(true, code).await?;
    assert_eq!("mylib", library);

    let libs: Vec<LibraryInfo> = client
        .function_list(FunctionListOptions::default().library_name_pattern("mylib"))
        .await?;
    assert_eq!(1, libs.len());
    assert_eq!("mylib", libs[0].library_name);

    let libs: Vec<LibraryInfo> = client
        .function_list(FunctionListOptions::default().library_name_pattern("otherlib"))
        .await?;
    assert!(libs.is_empty());

    Ok(())
}
