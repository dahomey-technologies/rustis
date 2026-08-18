//! Compiles the crate the way a downstream user does.
//!
//! Every other test lives in `src/tests/`, inside the crate, where `pub(crate)`
//! items are in scope and a `use crate::…` path reaches anything. That hides two
//! classes of breakage: an item that is public in name but unreachable by path,
//! and a command family implemented on one executor and forgotten on another —
//! the batch impl lists are hand-maintained, and a whole family was once
//! unusable in a pipeline and a transaction while the crate's own tests stayed
//! green.
//!
//! The bodies below are never run and most are never called: what is asserted is
//! that they **compile** against the published surface. That is deliberate — a
//! test needing a server would only run where one is up, and this file has to
//! fail on `cargo test --no-run`, on a laptop, with no Redis anywhere.

#![allow(dead_code)]

use rustis::{
    Result,
    client::{BatchPreparedCommand, Client, ExclusiveClient, IntoConfig, Pipeline, Transaction},
    commands::{
        BlockingCommands, GenericCommands, HashCommands, ListCommands, ServerCommands,
        SortedSetCommands, StringCommands, TransactionCommands, VectorSetCommands,
    },
    resp::cmd,
};

/// The families a pipeline must offer, one command each. Adding a family to
/// `Client` and forgetting it here is what this call list catches: `.queue()`
/// resolves only for `&mut Pipeline`, so a family implemented for the shared
/// reference — or not implemented at all — fails to compile on the line that
/// names it.
async fn pipeline_queues_every_family(client: &Client) -> Result<()> {
    let mut pipeline: Pipeline<'_> = client.create_pipeline();

    pipeline.set("key", "value").queue();
    pipeline.get::<String>("key").queue();
    pipeline.del("key").forget();
    pipeline.hset("hash", ("field", "value")).queue();
    pipeline.lpush("list", "value").queue();
    pipeline
        .zadd("zset", [(1.0, "member")], Default::default())
        .queue();
    pipeline
        .vadd("vset", None, &[1.0, 2.0], "element", Default::default())
        .queue();
    pipeline.dbsize().queue();

    let (_, _): (String, usize) = pipeline.execute().await?;
    Ok(())
}

/// The same list for a transaction, whose impl block is separate and was the
/// other half of the same defect.
async fn transaction_queues_every_family(client: &Client) -> Result<()> {
    let mut transaction: Transaction = client.create_transaction();

    transaction.set("key", "value").forget();
    transaction.get::<String>("key").queue();
    transaction.hset("hash", ("field", "value")).queue();
    transaction.lpush("list", "value").queue();
    transaction
        .zadd("zset", [(1.0, "member")], Default::default())
        .queue();
    transaction
        .vadd("vset", None, &[1.0, 2.0], "element", Default::default())
        .queue();
    transaction.dbsize().queue();

    let _: (String, usize) = transaction.execute().await?;
    Ok(())
}

/// The blocking and transaction families are reachable on an exclusive client
/// and on that one only. The negative half — that they are *absent* from
/// `Client` — is pinned by the `compile_fail` doctests, which are the only
/// place a compile error can be the assertion.
async fn exclusive_client_owns_the_reserved_families(config: impl IntoConfig) -> Result<()> {
    let client = ExclusiveClient::connect(config).await?;

    let _: Option<(String, String)> = client.blpop("list", 0.0).await?;
    client.watch("key").await?;
    client.unwatch().await?;

    // and every shared family too, through the same macro
    client.set("key", "value").await?;
    Ok(())
}

/// A client is clonable and an exclusive one is not; the conversion is the only
/// bridge, and it is fallible.
async fn the_two_client_shapes(config: impl IntoConfig) -> Result<()> {
    let client = Client::connect(config).await?;
    let _clone = client.clone();
    let _exclusive: ExclusiveClient = client.into_exclusive()?;
    Ok(())
}

/// The generic command API, with the key marked so cluster routing works.
async fn generic_commands(client: &Client) -> Result<()> {
    let _: String = client.send(cmd("GET").key("key"), None).await?;
    Ok(())
}

/// Error classification is reachable from outside: `ErrorKind` and
/// `ClientError` are both `#[non_exhaustive]`, so a downstream crate cannot
/// write these predicates itself and has to be given them.
#[test]
fn errors_can_be_classified_from_outside() {
    use rustis::{Error, ErrorKind, TimeoutKind};

    let error: Error = ErrorKind::Timeout(TimeoutKind::Command).into();

    assert!(error.is_timeout());
    assert!(error.is_retryable());
    assert!(!error.is_server_error());
    assert!(matches!(error.kind(), ErrorKind::Timeout(_)));
}

/// A URI is parsed without a server, and an unknown parameter is an error that
/// names itself — the behaviour a downstream crate builds its own config
/// validation on.
#[test]
fn a_config_round_trips_through_its_uri() {
    use rustis::client::{Config, IntoConfig};

    let config: Config = "redis://127.0.0.1:6379/1?command_timeout=5000"
        .into_config()
        .expect("a well-formed URI parses");
    assert_eq!(5000, config.command_timeout.as_millis());

    let error = "redis://127.0.0.1:6379?not_a_parameter=1"
        .into_config()
        .expect_err("an unknown parameter is rejected");
    assert!(
        error.to_string().contains("not_a_parameter"),
        "the error must name the offending parameter, got: {error}"
    );
}
