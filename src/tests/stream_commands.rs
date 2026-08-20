use crate::{
    Result,
    commands::{
        ConsumerGroupOptions, StreamCommands, XAddOptions, XCfgSetOptions, XClaimOptions,
        XNackMode, XNackOptions, XSetIdOptions, XTrimOptions,
    },
    tests::TestClient,
};

#[test]
fn xnack_args() {
    let cmd = TestClient
        .xnack(
            "mystream",
            "mygroup",
            XNackMode::Fatal,
            ["1-1", "2-2"],
            XNackOptions::default().retry_count(3).force(),
        )
        .command;
    assert_eq!(
        "XNACK mystream mygroup FATAL IDS 2 1-1 2-2 RETRYCOUNT 3 FORCE",
        cmd.to_string()
    );

    let cmd = TestClient
        .xnack(
            "mystream",
            "mygroup",
            XNackMode::Silent,
            "1-1",
            XNackOptions::default(),
        )
        .command;
    assert_eq!("XNACK mystream mygroup SILENT IDS 1 1-1", cmd.to_string());
}

#[test]
fn xsetid_args() -> Result<()> {
    let cmd = TestClient
        .xsetid("key", "100-0", XSetIdOptions::default())
        .command;
    assert_eq!("XSETID key 100-0", &cmd.to_string());

    let cmd = TestClient
        .xsetid(
            "key",
            "100-0",
            XSetIdOptions::default()
                .entries_added(42)
                .max_deleted_id("7-3"),
        )
        .command;
    assert_eq!(
        "XSETID key 100-0 ENTRIESADDED 42 MAXDELETEDID 7-3",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn xclaim_lastid_args() -> Result<()> {
    let cmd = TestClient
        .xclaim::<()>(
            "key",
            "group",
            "consumer",
            0,
            "1-1",
            XClaimOptions::default().last_id("5-5"),
        )
        .command;
    assert_eq!(
        "XCLAIM key group consumer 0 1-1 LASTID 5-5",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn xadd_idmp_args() -> Result<()> {
    // The idempotency clause sits between the consumer-group policy and the
    // trimming clause, as the XADD grammar requires.
    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default()
                .no_mk_stream()
                .consumer_group_options(ConsumerGroupOptions::DelRef)
                .idmp("producer-1", "iid-1")
                .trim_options(XTrimOptions::max_len(None, 1000)),
        )
        .command;
    assert_eq!(
        "XADD key NOMKSTREAM DELREF IDMP producer-1 iid-1 MAXLEN 1000 * field value",
        &cmd.to_string()
    );

    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default().idmp_auto("producer-1"),
        )
        .command;
    assert_eq!(
        "XADD key IDMPAUTO producer-1 * field value",
        &cmd.to_string()
    );

    // The two modes are mutually exclusive: the last one set wins.
    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default()
                .idmp("producer-1", "iid-1")
                .idmp_auto("producer-1"),
        )
        .command;
    assert_eq!(
        "XADD key IDMPAUTO producer-1 * field value",
        &cmd.to_string()
    );

    let cmd = TestClient
        .xadd::<()>(
            "key",
            "*",
            [("field", "value")],
            XAddOptions::default()
                .idmp_auto("producer-1")
                .idmp("producer-1", "iid-1"),
        )
        .command;
    assert_eq!(
        "XADD key IDMP producer-1 iid-1 * field value",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn xcfgset_args() -> Result<()> {
    let cmd = TestClient
        .xcfgset(
            "key",
            XCfgSetOptions::default()
                .idmp_duration(300)
                .idmp_maxsize(50),
        )
        .command;
    assert_eq!(
        "XCFGSET key IDMP-DURATION 300 IDMP-MAXSIZE 50",
        &cmd.to_string()
    );

    let cmd = TestClient
        .xcfgset("key", XCfgSetOptions::default().idmp_maxsize(50))
        .command;
    assert_eq!("XCFGSET key IDMP-MAXSIZE 50", &cmd.to_string());

    Ok(())
}

#[test]
fn xautoclaim_args() -> Result<()> {
    let cmd = TestClient
        .xclaim::<()>(
            "key",
            "group",
            "consumer",
            1000,
            "1526569498055-0",
            XClaimOptions::default()
                .idle_time(100)
                .time(1000)
                .retry_count(12)
                .force()
                .just_id(),
        )
        .command;
    assert_eq!(
        "XCLAIM key group consumer 1000 1526569498055-0 IDLE 100 TIME 1000 RETRYCOUNT 12 FORCE JUSTID",
        &cmd.to_string()
    );

    Ok(())
}
