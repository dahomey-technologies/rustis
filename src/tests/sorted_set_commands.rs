use crate::{
    Result,
    commands::{SortedSetCommands, ZAggregate, ZRangeOptions},
    tests::TestClient,
};

#[test]
fn zaggregate_args() {
    // The aggregation value must be introduced by the AGGREGATE token.
    let cmd = TestClient
        .zinter::<()>(["key1", "key2"], None as Option<f64>, ZAggregate::Count)
        .command;
    assert_eq!("ZINTER 2 key1 key2 AGGREGATE COUNT", cmd.to_string());

    let cmd = TestClient
        .zunionstore("out", ["key1", "key2"], [2, 3], ZAggregate::Min)
        .command;
    assert_eq!(
        "ZUNIONSTORE out 2 key1 key2 WEIGHTS 2 3 AGGREGATE MIN",
        cmd.to_string()
    );

    // No aggregation means no token at all.
    let cmd = TestClient
        .zunion::<()>(["key1", "key2"], None as Option<f64>, None)
        .command;
    assert_eq!("ZUNION 2 key1 key2", cmd.to_string());
}

#[test]
fn zrange_reverse_emits_rev() -> Result<()> {
    // The server's token is `REV`; `REVERSE` is a syntax error.
    let cmd = TestClient
        .zrange::<()>("key", 0, -1, ZRangeOptions::default().reverse())
        .command;
    assert_eq!("ZRANGE key 0 -1 REV", &cmd.to_string());

    let cmd = TestClient
        .zrangestore("dst", "key", 0, -1, ZRangeOptions::default().reverse())
        .command;
    assert_eq!("ZRANGESTORE dst key 0 -1 REV", &cmd.to_string());

    Ok(())
}
