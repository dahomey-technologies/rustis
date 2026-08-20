//! The `command_future` tests that need a live Redis. The ones that need none stay in
//! `command_future.rs`.

//! The ergonomic API is the one the documentation teaches, so it is the one
//! that must not allocate: `client.get("k").await` goes through
//! `IntoFuture for PreparedCommand<&Client, R>`, and what that associated type
//! resolves to decides whether every awaited command costs a heap allocation
//! plus a virtual call. A named future keeps the state machine in the caller's
//! frame; a `BoxFuture` does not.

use crate::{
    Result,
    commands::{FlushingMode, ServerCommands, StringCommands},
    tests::get_test_client,
};
use serial_test::serial;
use std::future::IntoFuture;

/// A future does nothing until it is polled. The hand-written state machine
/// could easily send the command in `into_future` instead — nothing in a
/// `.await` call site would tell the difference, but a future built and
/// dropped, or a `select!` losing the race before the first poll, would silently
/// hit the server.
#[tokio::test]
#[serial]
async fn building_the_future_sends_nothing() -> Result<()> {
    use crate::commands::GenericCommands;

    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let future = client.set("key", "value").into_future();
    drop(future);

    assert_eq!(0, client.exists("key").await?);

    client.close().await?;

    Ok(())
}

/// The unboxed future must still be a future in the ways that matter: a
/// command awaited through the ergonomic API resolves against a live server.
#[tokio::test]
#[serial]
async fn unboxed_future_round_trips() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.set("key", "value").await?;
    let value: String = client.get("key").await?;
    assert_eq!("value", value);

    client.close().await?;

    Ok(())
}
