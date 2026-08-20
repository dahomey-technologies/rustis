//! Every command that writes its result somewhere must mark that destination as
//! a key.
//!
//! A destination added with `arg` takes no part in slot computation, so the
//! command routes on its source keys alone and the client-side
//! `MismatchedKeySlots` guard cannot see the destination at all: a destination
//! owned by another node is sent and refused by the server instead of being
//! refused locally, and where the sources are keyless the write lands on a
//! random node.
//!
//! The commands are built against a stand-in executor rather than a client, the
//! question being what the builder marks and not what a server answers.

use crate::{
    client::PreparedCommand,
    commands::{
        GenericCommands, SetCommands, SortOptions, SortedSetCommands, StringCommands, ZAggregate,
    },
};
use serde::de::DeserializeOwned;

/// Stands in for the executor a command is prepared against. The traits are
/// implemented for any `Sized` receiver, and nothing here reaches the network.
struct Executor;

impl<'a> SetCommands<'a> for &'a Executor {}
impl<'a> SortedSetCommands<'a> for &'a Executor {}
impl<'a> GenericCommands<'a> for &'a Executor {}
impl<'a> StringCommands<'a> for &'a Executor {}

/// The keys a prepared command marks for routing, in the order it wrote them.
fn keys<E, R: DeserializeOwned>(prepared: PreparedCommand<'_, E, R>) -> Vec<String> {
    prepared
        .command()
        .keys()
        .map(|key| String::from_utf8_lossy(key.as_ref()).into_owned())
        .collect()
}

#[test]
fn a_set_store_command_marks_its_destination() {
    let executor = &Executor;

    assert_eq!(
        vec!["dest", "src1", "src2"],
        keys(executor.sdiffstore("dest", ["src1", "src2"]))
    );
    assert_eq!(
        vec!["dest", "src1", "src2"],
        keys(executor.sinterstore("dest", ["src1", "src2"]))
    );
    // The reference: this one has always marked it.
    assert_eq!(
        vec!["dest", "src1", "src2"],
        keys(executor.sunionstore("dest", ["src1", "src2"]))
    );
}

#[test]
fn a_sorted_set_store_command_marks_its_destination() {
    let executor = &Executor;

    assert_eq!(
        vec!["dest", "src1", "src2"],
        keys(executor.zdiffstore("dest", ["src1", "src2"]))
    );
    assert_eq!(
        vec!["dest", "src1", "src2"],
        keys(executor.zinterstore("dest", ["src1", "src2"], None::<f64>, ZAggregate::Sum))
    );
    assert_eq!(
        vec!["dest", "src1", "src2"],
        keys(executor.zunionstore("dest", ["src1", "src2"], None::<f64>, ZAggregate::Sum))
    );
}

/// `SORT … STORE` differs in shape: the destination follows the `STORE` token
/// rather than opening the command, and the source is a single key.
#[test]
fn sort_and_store_marks_its_destination() {
    let executor = &Executor;

    assert_eq!(
        vec!["src", "dest"],
        keys(executor.sort_and_store("src", "dest", SortOptions::default()))
    );
}

/// `LCS` writes nothing, but it still reads two keys, and both must be routed
/// on: a second key in another slot is a `CROSSSLOT` the client can refuse
/// before the round trip. Its three forms take the same pair.
#[test]
fn every_lcs_form_marks_both_of_its_keys() {
    let executor = &Executor;

    assert_eq!(
        vec!["key1", "key2"],
        keys(executor.lcs::<String>("key1", "key2"))
    );
    assert_eq!(vec!["key1", "key2"], keys(executor.lcs_len("key1", "key2")));
    assert_eq!(
        vec!["key1", "key2"],
        keys(executor.lcs_idx("key1", "key2", None, false))
    );
}
