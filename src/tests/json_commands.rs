use crate::{
    commands::{JsonCommands, JsonFpType, JsonSetOptions, SetCondition},
    tests::TestClient,
};

#[test]
fn json_set_args() {
    let cmd = TestClient
        .json_set(
            "key",
            "$",
            "[1.0]",
            JsonSetOptions::default()
                .condition(SetCondition::NX)
                .fpha(JsonFpType::Bf16),
        )
        .command;
    assert_eq!("JSON.SET key $ [1.0] NX FPHA BF16", cmd.to_string());

    let cmd = TestClient.json_set("key", "$", "[1.0]", None).command;
    assert_eq!("JSON.SET key $ [1.0]", cmd.to_string());
}
