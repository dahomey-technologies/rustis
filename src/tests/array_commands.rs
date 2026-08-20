use crate::{
    commands::{
        ArGrep, ArGrepPredicate, ArInfoOptions, ArLastItemsOptions, ArOperation, ArrayCommands,
    },
    tests::TestClient,
};

#[test]
fn array_args() {
    let cmd = TestClient
        .arinfo("key", ArInfoOptions::default().full())
        .command;
    assert_eq!("ARINFO key FULL", cmd.to_string());

    let cmd = TestClient
        .arlastitems::<()>("key", 3, ArLastItemsOptions::default().rev())
        .command;
    assert_eq!("ARLASTITEMS key 3 REV", cmd.to_string());

    let cmd = TestClient.arscan::<()>("key", 0, 10, 5).command;
    assert_eq!("ARSCAN key 0 10 LIMIT 5", cmd.to_string());
    let cmd = TestClient.arscan::<()>("key", 0, 10, None).command;
    assert_eq!("ARSCAN key 0 10", cmd.to_string());

    let cmd = TestClient
        .arop::<()>("key", 0, 2, ArOperation::Match("10"))
        .command;
    assert_eq!("AROP key 0 2 MATCH 10", cmd.to_string());

    let cmd = TestClient.armset("key", [(0, "a"), (5, "b")]).command;
    assert_eq!("ARMSET key 0 a 5 b", cmd.to_string());

    let cmd = TestClient.ardelrange("key", [(0, 2), (10, 11)]).command;
    assert_eq!("ARDELRANGE key 0 2 10 11", cmd.to_string());

    let cmd = TestClient
        .argrep::<()>(
            "key",
            "-",
            "+",
            ArGrep::new(ArGrepPredicate::Glob("a*"))
                .predicate(ArGrepPredicate::Exact("b"))
                .and()
                .limit(3)
                .with_values()
                .nocase(),
        )
        .command;
    assert_eq!(
        "ARGREP key - + GLOB a* EXACT b AND LIMIT 3 WITHVALUES NOCASE",
        cmd.to_string()
    );
}
