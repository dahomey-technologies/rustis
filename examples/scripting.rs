//! Running Lua on the server: `EVAL`, `EVALSHA` and Functions.
//!
//! A script runs atomically on the server, so a read-decide-write cycle costs
//! one round trip and cannot interleave with another client. `EVALSHA` sends the
//! digest instead of the source, which is what makes it cheap to call in a loop.
//!
//! Keys go in `keys`, everything else in `args`: only the keys are hashed for
//! cluster routing, so a key passed as an argument routes the script to the
//! wrong node.
//!
//! ```sh
//! cargo run --example scripting
//! ```
use rustis::{
    Result,
    client::Client,
    commands::{ScriptingCommands, StringCommands},
};

/// Sets a key only if its current value matches, and reports what it did.
const COMPARE_AND_SET: &str = r"
    if redis.call('GET', KEYS[1]) == ARGV[1] then
        redis.call('SET', KEYS[1], ARGV[2])
        return 1
    end
    return 0
";

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::connect("127.0.0.1:6379").await?;
    client.set("script_key", "old").await?;

    // Inline: the source travels with every call.
    let changed: i64 = client
        .eval(COMPARE_AND_SET, ["script_key"], ["old", "new"])
        .await?;
    println!("changed: {changed}");

    // Cached: load once, then call by digest. The server keeps the script until
    // `SCRIPT FLUSH` or a restart, so a caller must be ready to reload it.
    let sha1: String = client.script_load(COMPARE_AND_SET).await?;
    let changed: i64 = client
        .evalsha(&sha1, ["script_key"], ["new", "newer"])
        .await?;
    println!("changed: {changed}");

    // Functions (Redis 7+): a named library registered on the server, so the
    // caller does not carry the source at all.
    let library = r"#!lua name=examples
        redis.register_function('bump', function(keys, args)
            return redis.call('INCRBY', keys[1], args[1])
        end)
    ";
    let _: String = client.function_load(true, library).await?;
    let total: i64 = client.fcall("bump", ["counter"], [7]).await?;
    println!("counter: {total}");

    Ok(())
}
