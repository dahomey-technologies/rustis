use crate::{
    Result,
    commands::{
        ArGrep, ArGrepPredicate, ArInfoOptions, ArLastItemsOptions, ArOperation, ArrayCommands,
        FlushingMode, GenericCommands, ServerCommands,
    },
    tests::{TestClient, get_test_client},
};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn arinsert_and_cursor() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // ARINSERT reports the last index it wrote, and the cursor follows.
    assert_eq!(2, client.arinsert("key", ["a", "b", "c"]).await?);
    assert_eq!(3, client.arnext("key").await?);
    assert_eq!(3, client.arlen("key").await?);
    assert_eq!(3, client.arcount("key").await?);

    assert!(client.arseek("key", 10).await?);
    assert_eq!(10, client.arinsert("key", "d").await?);
    assert_eq!(11, client.arnext("key").await?);

    // The gap between index 3 and 9 counts towards the length but holds nothing.
    assert_eq!(11, client.arlen("key").await?);
    assert_eq!(4, client.arcount("key").await?);

    // Seeking a key that does not exist changes nothing.
    assert!(!client.arseek("missing", 1).await?);

    Ok(())
}

#[tokio::test]
#[serial]
async fn arset_armset_and_get() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    // Both setters report how many slots were empty before the call.
    assert_eq!(3, client.arset("key", 2, ["a", "b", "c"]).await?);
    assert_eq!(1, client.arset("key", 3, ["B", "C", "d"]).await?);
    assert_eq!(2, client.armset("key", [(0, "z"), (100, "far")]).await?);

    let value: Option<String> = client.arget("key", 3).await?;
    assert_eq!(Some("B".to_owned()), value);
    let value: Option<String> = client.arget("key", 1).await?;
    assert_eq!(None, value);

    let values: Vec<Option<String>> = client.armget("key", [0, 1, 100]).await?;
    assert_eq!(
        vec![Some("z".to_owned()), None, Some("far".to_owned())],
        values
    );

    let values: Vec<Option<String>> = client.argetrange("key", 0, 3).await?;
    assert_eq!(
        vec![
            Some("z".to_owned()),
            None,
            Some("a".to_owned()),
            Some("B".to_owned())
        ],
        values
    );

    // ARSCAN skips the gaps instead of reporting them.
    let entries: Vec<(usize, String)> = client.arscan("key", 0, 100, None).await?;
    assert_eq!(
        vec![
            (0, "z".to_owned()),
            (2, "a".to_owned()),
            (3, "B".to_owned()),
            (4, "C".to_owned()),
            (5, "d".to_owned()),
            (100, "far".to_owned())
        ],
        entries
    );

    let entries: Vec<(usize, String)> = client.arscan("key", 0, 100, 2).await?;
    assert_eq!(vec![(0, "z".to_owned()), (2, "a".to_owned())], entries);

    Ok(())
}

#[tokio::test]
#[serial]
async fn ardel_and_ardelrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.arset("key", 0, ["a", "b", "c", "d", "e"]).await?;

    // Only the slots that held something are counted.
    assert_eq!(2, client.ardel("key", [0, 1, 99]).await?);
    assert_eq!(3, client.arcount("key").await?);

    client.arset("key", 10, ["x", "y"]).await?;
    // Two ranges in one call: indices 2, 3, 4 and 10, 11.
    assert_eq!(5, client.ardelrange("key", [(2, 4), (10, 11)]).await?);
    assert_eq!(0, client.arcount("key").await?);

    Ok(())
}

#[tokio::test]
#[serial]
async fn arlastitems() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client.arinsert("key", ["a", "b", "c", "d"]).await?;

    let values: Vec<String> = client
        .arlastitems("key", 2, ArLastItemsOptions::default())
        .await?;
    assert_eq!(vec!["c".to_owned(), "d".to_owned()], values);

    let values: Vec<String> = client
        .arlastitems("key", 2, ArLastItemsOptions::default().rev())
        .await?;
    assert_eq!(vec!["d".to_owned(), "c".to_owned()], values);

    Ok(())
}

#[tokio::test]
#[serial]
async fn arring() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    assert_eq!(0, client.arring("key", 3, "v0").await?);
    assert_eq!(2, client.arring("key", 3, ["v1", "v2"]).await?);

    // The window is full, so the next value wraps onto the oldest slot.
    assert_eq!(0, client.arring("key", 3, "v3").await?);
    assert_eq!(Some("v3".to_owned()), client.arget("key", 0).await?);
    assert_eq!(3, client.arcount("key").await?);

    let values: Vec<String> = client
        .arlastitems("key", 3, ArLastItemsOptions::default())
        .await?;
    assert_eq!(
        vec!["v1".to_owned(), "v2".to_owned(), "v3".to_owned()],
        values
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn arop() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .armset("key", [(0, "10"), (1, "20"), (2, "30")])
        .await?;

    let sum: Option<String> = client.arop("key", 0, 2, ArOperation::Sum).await?;
    assert_eq!(Some("60".to_owned()), sum);
    let min: Option<String> = client.arop("key", 0, 2, ArOperation::Min).await?;
    assert_eq!(Some("10".to_owned()), min);
    let max: Option<String> = client.arop("key", 0, 2, ArOperation::Max).await?;
    assert_eq!(Some("30".to_owned()), max);

    let matched: usize = client.arop("key", 0, 2, ArOperation::Match("10")).await?;
    assert_eq!(1, matched);
    let used: usize = client.arop("key", 0, 2, ArOperation::Used).await?;
    assert_eq!(3, used);

    client.del("flags").await?;
    client
        .armset("flags", [(0, "255"), (1, "15"), (2, "240")])
        .await?;
    let and: i64 = client.arop("flags", 0, 2, ArOperation::And).await?;
    assert_eq!(0, and);
    let or: i64 = client.arop("flags", 0, 2, ArOperation::Or).await?;
    assert_eq!(255, or);
    let xor: i64 = client.arop("flags", 0, 2, ArOperation::Xor).await?;
    assert_eq!(0, xor);

    // Nothing to aggregate over gives nil, not zero.
    let sum: Option<String> = client.arop("key", 50, 60, ArOperation::Sum).await?;
    assert_eq!(None, sum);

    Ok(())
}

#[tokio::test]
#[serial]
async fn argrep() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .armset(
            "log",
            [
                (0, "boot: ok"),
                (1, "warn: disk"),
                (2, "ERROR: cpu"),
                (3, "info: ready"),
                (4, "error: net"),
            ],
        )
        .await?;

    let indices: Vec<usize> = client
        .argrep(
            "log",
            "-",
            "+",
            ArGrep::new(ArGrepPredicate::Match("error")).nocase(),
        )
        .await?;
    assert_eq!(vec![2, 4], indices);

    // WITHVALUES turns the reply into flat index/value pairs.
    let matches: Vec<(usize, String)> = client
        .argrep(
            "log",
            "-",
            "+",
            ArGrep::new(ArGrepPredicate::Match("error"))
                .nocase()
                .with_values(),
        )
        .await?;
    assert_eq!(
        vec![(2, "ERROR: cpu".to_owned()), (4, "error: net".to_owned())],
        matches
    );

    // Several predicates, combined with OR by default.
    let indices: Vec<usize> = client
        .argrep(
            "log",
            0,
            4,
            ArGrep::new(ArGrepPredicate::Glob("warn:*"))
                .predicate(ArGrepPredicate::Glob("error:*")),
        )
        .await?;
    assert_eq!(vec![1, 4], indices);

    // Asking for OR explicitly emits the token and keeps the same result.
    let indices: Vec<usize> = client
        .argrep(
            "log",
            0,
            4,
            ArGrep::new(ArGrepPredicate::Glob("warn:*"))
                .predicate(ArGrepPredicate::Glob("error:*"))
                .or(),
        )
        .await?;
    assert_eq!(vec![1, 4], indices);

    // AND narrows them instead.
    let indices: Vec<usize> = client
        .argrep(
            "log",
            0,
            4,
            ArGrep::new(ArGrepPredicate::Match("error"))
                .predicate(ArGrepPredicate::Glob("*cpu"))
                .nocase()
                .and(),
        )
        .await?;
    assert_eq!(vec![2], indices);

    let indices: Vec<usize> = client
        .argrep(
            "log",
            0,
            4,
            ArGrep::new(ArGrepPredicate::Re("^[A-Za-z]+: (cpu|net)$")).nocase(),
        )
        .await?;
    assert_eq!(vec![2, 4], indices);

    let indices: Vec<usize> = client
        .argrep(
            "log",
            0,
            4,
            ArGrep::new(ArGrepPredicate::Exact("info: ready")),
        )
        .await?;
    assert_eq!(vec![3], indices);

    let indices: Vec<usize> = client
        .argrep(
            "log",
            "-",
            "+",
            ArGrep::new(ArGrepPredicate::Match("error"))
                .nocase()
                .limit(1),
        )
        .await?;
    assert_eq!(vec![2], indices);

    // A reversed range walks the indices the other way.
    let indices: Vec<usize> = client
        .argrep(
            "log",
            "+",
            "-",
            ArGrep::new(ArGrepPredicate::Match("error")).nocase(),
        )
        .await?;
    assert_eq!(vec![4, 2], indices);

    Ok(())
}

#[tokio::test]
#[serial]
async fn arinfo() -> Result<()> {
    let client = get_test_client().await?;
    client.flushdb(FlushingMode::Sync).await?;

    client
        .armset("key", [(0, "a"), (1, "b"), (100, "c")])
        .await?;
    client.arinsert("key", "d").await?;

    let info = client.arinfo("key", ArInfoOptions::default()).await?;
    assert_eq!(3, info.count);
    assert_eq!(101, info.len);
    assert_eq!(1, info.next_insert_index);
    assert_eq!(1, info.slices);
    // The per-slice statistics are reported only under FULL.
    assert_eq!(None, info.dense_slices);
    assert_eq!(None, info.avg_dense_fill);

    let info = client
        .arinfo("key", ArInfoOptions::default().full())
        .await?;
    assert_eq!(3, info.count);
    assert_eq!(Some(0), info.dense_slices);
    assert_eq!(Some(1), info.sparse_slices);
    assert!(info.avg_sparse_size.is_some());

    Ok(())
}

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
