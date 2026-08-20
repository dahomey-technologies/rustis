use crate::{
    commands::{GenericCommands, MigrateOptions},
    tests::TestClient,
};

/// `AUTH password` and `AUTH2 username password` are the two authentication
/// forms of MIGRATE. The test servers need no password, so the wire form is
/// asserted instead.
#[test]
fn migrate_auth_args() {
    let cmd = TestClient
        .migrate(
            "host",
            6379,
            "key",
            0,
            1000,
            MigrateOptions::default().auth("password"),
        )
        .command;
    assert_eq!(
        "MIGRATE host 6379 key 0 1000 AUTH password",
        cmd.to_string()
    );

    let cmd = TestClient
        .migrate(
            "host",
            6379,
            "key",
            0,
            1000,
            MigrateOptions::default().auth2("username", "password"),
        )
        .command;
    assert_eq!(
        "MIGRATE host 6379 key 0 1000 AUTH2 username password",
        cmd.to_string()
    );

    // With KEYS the single-key slot is an empty string and the keys follow.
    let cmd = TestClient
        .migrate(
            "host",
            6379,
            "",
            0,
            1000,
            MigrateOptions::default().replace().key("key1").key("key2"),
        )
        .command;
    assert_eq!(
        "MIGRATE host 6379  0 1000 REPLACE KEYS key1 key2",
        cmd.to_string()
    );
}
