use crate::{
    commands::{
        FailOverOptions, HotKeysMetric, HotKeysStartOptions, ModuleLoadexOptions, ServerCommands,
        ShutdownOptions,
    },
    tests::TestClient,
};

#[test]
fn module_loadex_args() {
    let cmd = TestClient
        .module_loadex("/path/mod.so", ModuleLoadexOptions::default())
        .command;
    assert_eq!("MODULE LOADEX /path/mod.so", cmd.to_string());

    // CONFIG is repeated once per name/value pair, and ARGS closes the command.
    let cmd = TestClient
        .module_loadex(
            "/path/mod.so",
            ModuleLoadexOptions::default()
                .config("timeout", "100")
                .config("mode", "fast")
                .args(["arg1", "arg2"]),
        )
        .command;
    assert_eq!(
        "MODULE LOADEX /path/mod.so CONFIG timeout 100 CONFIG mode fast ARGS arg1 arg2",
        cmd.to_string()
    );
}

/// The forms that do take the server down can only have their wire form
/// checked, against the syntax published for `SHUTDOWN`.
#[test]
fn hotkeys_start_args() {
    let cmd = TestClient
        .hotkeys_start(
            [HotKeysMetric::Cpu, HotKeysMetric::Net],
            HotKeysStartOptions::default(),
        )
        .command;
    assert_eq!("HOTKEYS START METRICS 2 CPU NET", cmd.to_string());

    // `SLOTS` carries its own count and is only accepted in cluster mode, which
    // the integration test cannot reach.
    let cmd = TestClient
        .hotkeys_start(
            [HotKeysMetric::Cpu],
            HotKeysStartOptions::default()
                .count(5)
                .duration(60)
                .sample(10)
                .slots([0, 100, 16383]),
        )
        .command;
    assert_eq!(
        "HOTKEYS START METRICS 1 CPU COUNT 5 DURATION 60 SAMPLE 10 SLOTS 3 0 100 16383",
        cmd.to_string()
    );

    // An empty slot list still emits the clause, so the server explains the
    // mistake rather than the client silently tracking everything.
    let cmd = TestClient
        .hotkeys_start(
            [HotKeysMetric::Cpu],
            HotKeysStartOptions::default().slots([]),
        )
        .command;
    assert_eq!("HOTKEYS START METRICS 1 CPU SLOTS 0", cmd.to_string());
}

#[test]
fn shutdown_command() {
    let cmd = TestClient.shutdown(ShutdownOptions::default()).command;
    assert_eq!("SHUTDOWN", cmd.to_string());

    let cmd = TestClient
        .shutdown(ShutdownOptions::default().save(false).now().force())
        .command;
    assert_eq!("SHUTDOWN NOSAVE NOW FORCE", cmd.to_string());

    let cmd = TestClient
        .shutdown(ShutdownOptions::default().save(true))
        .command;
    assert_eq!("SHUTDOWN SAVE", cmd.to_string());
}

/// `FAILOVER [TO host port [FORCE]] [ABORT] [TIMEOUT milliseconds]`. TO promotes
/// a designated replica, so it cannot be sent against the shared test server;
/// the wire form is asserted instead.
#[test]
fn failover_to_args() {
    let cmd = TestClient
        .failover(FailOverOptions::default().to("127.0.0.1", 6379))
        .command;
    assert_eq!("FAILOVER TO 127.0.0.1 6379", cmd.to_string());

    let cmd = TestClient
        .failover(
            FailOverOptions::default()
                .to("127.0.0.1", 6379)
                .force()
                .timeout(1000),
        )
        .command;
    assert_eq!(
        "FAILOVER TO 127.0.0.1 6379 FORCE TIMEOUT 1000",
        cmd.to_string()
    );
}
