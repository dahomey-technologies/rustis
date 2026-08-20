//! The `value` tests that need a live Redis. The ones that need none stay in
//! `value.rs`.

use crate::{
    Result,
    commands::{FlushingMode, ServerCommands, SetCommands},
    tests::get_test_client,
};
use serial_test::serial;
use std::collections::{BTreeSet, HashSet};

#[tokio::test]
#[serial]
async fn from_single_value_array() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .sadd("key", ["member1", "member2", "member3"])
        .await?;

    let members: Vec<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains(&"member1".to_owned()));
    assert!(members.contains(&"member2".to_owned()));
    assert!(members.contains(&"member3".to_owned()));

    let members: HashSet<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains("member1"));
    assert!(members.contains("member2"));
    assert!(members.contains("member3"));

    let members: BTreeSet<String> = client.smembers("key").await?;
    assert_eq!(3, members.len());
    assert!(members.contains("member1"));
    assert!(members.contains("member2"));
    assert!(members.contains("member3"));

    Ok(())
}
