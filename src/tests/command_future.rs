//! The ergonomic API is the one the documentation teaches, so it is the one
//! that must not allocate: `client.get("k").await` goes through
//! `IntoFuture for PreparedCommand<&Client, R>`, and what that associated type
//! resolves to decides whether every awaited command costs a heap allocation
//! plus a virtual call. A named future keeps the state machine in the caller's
//! frame; a `BoxFuture` does not.

use crate::{
    Result,
    client::{Client, CommandFuture, PreparedCommand},
    commands::{FlushingMode, ServerCommands, StringCommands},
    tests::get_test_client,
};
use serial_test::serial;
use std::{any::type_name, future::IntoFuture, mem::size_of};

type ErgonomicFuture<'a> = <PreparedCommand<'a, &'a Client, String> as IntoFuture>::IntoFuture;

/// The awaited type is the crate's own future, not a trait object behind a
/// pointer. `type_name` is what tells the two apart from a test.
#[test]
fn ergonomic_api_awaits_a_named_future() {
    let name = type_name::<ErgonomicFuture<'static>>();
    assert!(
        name.contains("CommandFuture"),
        "the ergonomic API awaits `{name}`"
    );
    assert!(
        !name.contains("Pin<") && !name.contains("Box<"),
        "the ergonomic API awaits a boxed future: `{name}`"
    );
}

/// A boxed future is one fat pointer wide whatever it carries. Ours holds the
/// oneshot receiver and the timeout, so it is wider — the cheap structural
/// check that the state machine really lives inline.
#[test]
fn awaited_future_carries_its_state_inline() {
    assert!(
        size_of::<ErgonomicFuture<'static>>() > size_of::<*const ()>() * 2,
        "the awaited future is no wider than a boxed trait object"
    );
    assert_eq!(
        size_of::<ErgonomicFuture<'static>>(),
        size_of::<CommandFuture<'static, String>>()
    );
}

/// `BoxFuture` carries a `Send` bound; dropping the box must not drop the
/// property, or a client awaited inside `tokio::spawn` stops compiling.
#[test]
fn awaited_future_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<ErgonomicFuture<'static>>();
}

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
