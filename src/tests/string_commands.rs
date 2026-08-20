use crate::{
    commands::{IncrExOptions, StringCommands},
    tests::TestClient,
};

#[test]
fn increx_args() {
    let cmd = TestClient
        .increx::<()>(
            "key",
            IncrExOptions::by_int(5)
                .lbound_int(0)
                .ubound_int(100)
                .saturate()
                .ex(60)
                .enx(),
        )
        .command;
    assert_eq!(
        "INCREX key BYINT 5 LBOUND 0 UBOUND 100 SATURATE EX 60 ENX",
        cmd.to_string()
    );

    let cmd = TestClient
        .increx::<()>(
            "key",
            IncrExOptions::by_float(0.5).lbound_float(-1.5).persist(),
        )
        .command;
    assert_eq!(
        "INCREX key BYFLOAT 0.5 LBOUND -1.5 PERSIST",
        cmd.to_string()
    );

    let cmd = TestClient
        .increx::<()>("key", IncrExOptions::default())
        .command;
    assert_eq!("INCREX key", cmd.to_string());
}
