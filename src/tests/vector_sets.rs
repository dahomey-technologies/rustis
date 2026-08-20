use crate::{
    Result,
    commands::{QuantizationOptions, VAddOptions, VSimOptions, VectorOrElement, VectorSetCommands},
    tests::TestClient,
};

#[test]
fn vadd_args() -> Result<()> {
    let cmd = TestClient
        .vadd(
            "key",
            12,
            &[1.0, 2.0, 3.0],
            "element",
            VAddOptions::default()
                .cas()
                .quantization(QuantizationOptions::NoQuant)
                .ef(12)
                .set_attr("{\"type\": \"fruit\", \"color\": \"red\"}")
                .m(12),
        )
        .command;
    assert_eq!(
        "VADD key 12 FP32 \0\0�?\0\0\0@\0\0@@ element CAS NOQUANT EF 12 SETATTR {\"type\": \"fruit\", \"color\": \"red\"} M 12",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn vsim_with_attributes_args() {
    // The server's token is `WITHATTRIBS`; `WITHATTRIBUTES` is a syntax error.
    let cmd = TestClient
        .vsim::<()>(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default().with_scores().with_attributes(),
        )
        .command;
    assert_eq!("VSIM key ELE apple WITHSCORES WITHATTRIBS", cmd.to_string());
}
