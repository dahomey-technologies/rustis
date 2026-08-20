//! The ergonomic API is the one the documentation teaches, so it is the one
//! that must not allocate: `client.get("k").await` goes through
//! `IntoFuture for PreparedCommand<&Client, R>`, and what that associated type
//! resolves to decides whether every awaited command costs a heap allocation
//! plus a virtual call. A named future keeps the state machine in the caller's
//! frame; a `BoxFuture` does not.

use crate::client::{Client, CommandFuture, PreparedCommand};
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
