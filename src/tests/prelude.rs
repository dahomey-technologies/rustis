//! What one `use rustis::prelude::*` has to be enough for.

use crate::{
    Result,
    prelude::*,
    tests::{
        fake_server::{FakeServer, duplex_config},
        log_try_init,
    },
};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    sync::{Arc, atomic::AtomicUsize},
};

/// The prelude alone resolves what a program calls on a client: a command, its
/// `forget` form, and its queued form on a pipeline.
///
/// Nothing below names a trait or an executor. A prelude that stops carrying one
/// fails to compile here.
#[tokio::test]
async fn the_prelude_alone_resolves_the_command_methods() -> Result<()> {
    log_try_init();
    let client = Client::connect(duplex_config(
        FakeServer::new()
            .reply("PING", b"+PONG\r\n")
            .reply("SET", b"+OK\r\n")
            .reply("GET", b"$3\r\nabc\r\n"),
        Arc::new(AtomicUsize::new(0)),
    ))
    .await?;

    assert_eq!("PONG", client.ping::<String>(()).await?);
    client.set("key", "abc").forget()?;

    let mut pipeline = client.create_pipeline();
    pipeline.get::<()>("key").queue();
    let value: String = pipeline.execute().await?;
    assert_eq!("abc", value);

    Ok(())
}

/// The prelude names the types a program writes into its own signatures.
///
/// A handler taking a message, a helper filling a pipeline, a task owning half a
/// split stream: each names its type, and a `Client` handing the value out does
/// not spare the import.
#[test]
fn the_prelude_names_the_types_a_program_writes_in_its_signatures() {
    fn handler(
        _exclusive: &ExclusiveClient,
        _pipeline: &mut Pipeline<'_>,
        _transaction: &mut Transaction,
        _stream: &PubSubStream,
        _sink: &PubSubSplitSink,
        _split_stream: &PubSubSplitStream,
        _message: &PubSubMessage,
    ) {
    }

    // Naming it is what puts the signature above under the compiler.
    let _ = handler;
}

/// Every command family a caller can import is in the prelude.
///
/// The prelude names its families one by one, so a family added under
/// `src/commands` and forgotten there would compile: only a caller writing the
/// `use` line the prelude exists to spare them would notice.
#[test]
fn every_public_command_family_is_in_the_prelude() {
    let exported = reexported_command_families();
    assert!(!exported.is_empty(), "the prelude re-exports no family");

    let public = public_command_families();
    let missing: Vec<&String> = public
        .iter()
        .filter(|family| !exported.contains(family.as_str()))
        .collect();

    assert!(
        missing.is_empty(),
        "src/prelude.rs does not re-export {missing:?}"
    );
}

/// Reads a file under the crate root.
fn source(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

/// The command families a caller outside the crate can name: one trait per
/// module `src/commands/mod.rs` re-exports publicly and unconditionally.
fn public_command_families() -> Vec<String> {
    let commands = source("src/commands/mod.rs");
    let mut families = Vec::new();

    for line in commands.lines() {
        let Some(module) = line
            .strip_prefix("pub use ")
            .and_then(|rest| rest.strip_suffix("::*;"))
        else {
            continue;
        };
        // A `#[cfg(test)]` module is re-exported on the line below its
        // attribute, so the attribute is what tells the two apart.
        if commands.contains(&format!("#[cfg(test)]\npub use {module}::*;")) {
            continue;
        }

        families.extend(
            source(&format!("src/commands/{module}.rs"))
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("pub trait ")
                        .map(|rest| rest.split(['<', ':', ' ']).next().unwrap().to_owned())
                }),
        );
    }

    families
}

/// The families named in the prelude's single `pub use crate::commands` item.
///
/// Only that item counts: a family named in the module documentation, or in a
/// doc link, is not re-exported.
fn reexported_command_families() -> HashSet<String> {
    let prelude = source("src/prelude.rs");
    let start = prelude.find("pub use crate::commands::{").unwrap();
    let item = &prelude[start..];
    let end = item.find("};").unwrap();

    item[..end]
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(str::to_owned)
        .collect()
}
