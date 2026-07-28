use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::{
    Result,
    commands::{
        FlushingMode, QuantizationOptions, ServerCommands, VAddOptions, VSimOptions,
        VectorOrElement, VectorSetCommands,
    },
    tests::{TestClient, get_test_client},
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vadd() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    Ok(())
}

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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vcard() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result = client.vcard("key").await?;
    assert_eq!(1, result);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vdim() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result = client.vdim("key").await?;
    assert_eq!(3, result);

    Ok(())
}

fn vec_f32_approx_eq(a: &[f32], b: &[f32], epsilon: f32) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < epsilon)
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vemb() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result: Vec<f32> = client.vemb("key", "element").await?;
    assert!(vec_f32_approx_eq(&[0.1, 1.2, 0.5], &result, 0.01));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vgetattr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result = client
        .vsetattr("key", "element", r#"{"key":"value"}"#)
        .await?;
    assert!(result);

    let json: String = client.vgetattr("key", "element").await?;
    assert_eq!(r#"{"key":"value"}"#, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vinfo() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result = client.vinfo("key").await?;
    assert_eq!("int8", result.quant_type);
    assert_eq!(3, result.vector_dim);
    assert_eq!(1, result.size);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vlinks() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result: Vec<Option<String>> = client.vlinks("key", "element").await?;
    assert!(result.into_iter().all(|r| r.is_none()));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vlinks_with_score() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    for i in 0..5 {
        client
            .vadd(
                "key",
                None,
                &[0.1 * i as f32, 1.2, 0.5],
                format!("element{i}"),
                VAddOptions::default(),
            )
            .await?;
    }

    // One entry per HNSW layer, each holding the neighbours of `element0` with
    // their similarity score. The upper layers are empty on a set this small.
    let layers: Vec<HashMap<String, f64>> = client.vlinks_with_score("key", "element0").await?;
    assert!(!layers.is_empty());

    let neighbours = layers.last().unwrap();
    assert_eq!(4, neighbours.len());
    assert!(neighbours.keys().all(|n| n != "element0"));
    assert!(neighbours.values().all(|s| (0. ..=1.).contains(s)));

    Ok(())
}

fn are_all_unique<T: Eq + Hash>(vec: &[T]) -> bool {
    let mut set = HashSet::with_capacity(vec.len());
    vec.iter().all(|item| set.insert(item))
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vrandmember() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "vset",
            None,
            &[3.0, 1.0, 0.0, 0.0],
            "elem1",
            VAddOptions::default(),
        )
        .await?;

    client
        .vadd(
            "vset",
            None,
            &[3.0, 0.0, 1.0, 0.0],
            "elem2",
            VAddOptions::default(),
        )
        .await?;

    client
        .vadd(
            "vset",
            None,
            &[3.0, 0.0, 0.0, 1.0],
            "elem3",
            VAddOptions::default(),
        )
        .await?;

    let result: Vec<String> = client.vrandmember("vset", 1).await?;
    assert_eq!(1, result.len());
    assert!(result[0] == "elem1" || result[0] == "elem2" || result[0] == "elem3");

    let result: Vec<String> = client.vrandmember("vset", 2).await?;
    assert_eq!(2, result.len());
    assert!(result[0] == "elem1" || result[0] == "elem2" || result[0] == "elem3");
    assert!(result[1] == "elem1" || result[1] == "elem2" || result[1] == "elem3");
    assert!(are_all_unique(&result));

    let result: Vec<String> = client.vrandmember("vset", -3).await?;
    assert_eq!(3, result.len());
    assert!(result[0] == "elem1" || result[0] == "elem2" || result[0] == "elem3");
    assert!(result[1] == "elem1" || result[1] == "elem2" || result[1] == "elem3");
    assert!(result[2] == "elem1" || result[2] == "elem2" || result[2] == "elem3");

    let result: Vec<String> = client.vrandmember("vset", 10).await?;
    assert_eq!(3, result.len());
    assert!(result[0] == "elem1" || result[0] == "elem2" || result[0] == "elem3");
    assert!(result[1] == "elem1" || result[1] == "elem2" || result[1] == "elem3");
    assert!(result[2] == "elem1" || result[2] == "elem2" || result[2] == "elem3");
    assert!(are_all_unique(&result));

    let result: Vec<String> = client.vrandmember("nonexistent", 1).await?;
    assert!(result.is_empty());

    let result: Vec<String> = client.vrandmember("nonexistent", 3).await?;
    assert!(result.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vrem() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "vset",
            None,
            &[3.0, 1.0, 0.0, 1.0],
            "bar",
            VAddOptions::default(),
        )
        .await?;

    client.vrem("vset", "bar").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vsetattr() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "element",
            VAddOptions::default(),
        )
        .await?;

    let result = client
        .vsetattr(
            "key",
            "element",
            r#"{\"type\": \"fruit\", \"color\": \"red\"}"#,
        )
        .await?;
    assert!(result);

    let json: Option<String> = client.vgetattr("key", "element").await?;
    assert_eq!(
        Some(r#"{\"type\": \"fruit\", \"color\": \"red\"}"#.to_string()),
        json
    );

    let result = client.vsetattr("key", "element", "").await?;
    assert!(result);

    let json: Option<String> = client.vgetattr("key", "element").await?;
    assert_eq!(None, json);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vsim() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "apple",
            VAddOptions::default(),
        )
        .await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "apples",
            VAddOptions::default(),
        )
        .await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "pear",
            VAddOptions::default(),
        )
        .await?;

    let result: Vec<String> = client
        .vsim(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default(),
        )
        .await?;
    assert_eq!(3, result.len());
    assert!(result.contains(&"apple".to_string()));
    assert!(result.contains(&"apples".to_string()));
    assert!(result.contains(&"pear".to_string()));

    let result: Vec<String> = client
        .vsim(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default().count(2),
        )
        .await?;
    assert_eq!(2, result.len());
    assert!(result[0] == "apple" || result[0] == "apples" || result[0] == "pear");
    assert!(result[1] == "apple" || result[1] == "apples" || result[1] == "pear");

    let result: Vec<(String, f64)> = client
        .vsim(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default().with_scores(),
        )
        .await?;
    assert_eq!(3, result.len());
    assert!(result[0].0 == "apple" || result[0].0 == "apples" || result[0].0 == "pear");
    assert!(result[1].0 == "apple" || result[1].0 == "apples" || result[1].0 == "pear");
    assert!(result[2].0 == "apple" || result[2].0 == "apples" || result[2].0 == "pear");

    let result: Vec<String> = client
        .vsim(
            "movies",
            VectorOrElement::Vector(&[0.5, 0.8, 0.2]),
            VSimOptions::default().filter(".year >= 1980 and .rating > 7"),
        )
        .await?;
    assert!(result.is_empty());

    let result: Vec<String> = client
        .vsim(
            "vset",
            VectorOrElement::Vector(&[0.0, 0.0, 0.0]),
            VSimOptions::default().filter(".year > 2000").filter_ef(500),
        )
        .await?;
    assert!(result.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vsim_with_attributes() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "apple",
            VAddOptions::default(),
        )
        .await?;
    client
        .vadd(
            "key",
            None,
            &[0.9, 0.1, 0.2],
            "pear",
            VAddOptions::default(),
        )
        .await?;
    client
        .vsetattr("key", "apple", r#"{"color":"red"}"#)
        .await?;

    // Each element comes back with its attributes, or nil when it has none.
    let result: Vec<(String, Option<String>)> = client
        .vsim(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default().with_attributes(),
        )
        .await?;
    assert_eq!(2, result.len());
    let apple = result.iter().find(|(e, _)| e == "apple").unwrap();
    assert_eq!(Some(r#"{"color":"red"}"#.to_owned()), apple.1);
    let pear = result.iter().find(|(e, _)| e == "pear").unwrap();
    assert_eq!(None, pear.1);

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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vismember() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .vadd(
            "key",
            None,
            &[0.1, 1.2, 0.5],
            "apple",
            VAddOptions::default(),
        )
        .await?;

    assert!(client.vismember("key", "apple").await?);
    assert!(!client.vismember("key", "pear").await?);
    // A missing key is not an error, just an absent element.
    assert!(!client.vismember("unknown", "apple").await?);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vrange() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    for element in ["apple", "banana", "cherry"] {
        client
            .vadd(
                "key",
                None,
                &[0.1, 1.2, 0.5],
                element,
                VAddOptions::default(),
            )
            .await?;
    }

    // The range is lexicographic over element names, not over vectors.
    let result: Vec<String> = client.vrange("key", "-", "+", None).await?;
    assert_eq!(
        vec!["apple".to_owned(), "banana".to_owned(), "cherry".to_owned()],
        result
    );

    let result: Vec<String> = client.vrange("key", "[banana", "+", None).await?;
    assert_eq!(vec!["banana".to_owned(), "cherry".to_owned()], result);

    // The optional count caps the number of elements returned.
    let result: Vec<String> = client.vrange("key", "-", "+", 2).await?;
    assert_eq!(vec!["apple".to_owned(), "banana".to_owned()], result);

    let result: Vec<String> = client.vrange("unknown", "-", "+", None).await?;
    assert!(result.is_empty());

    Ok(())
}

/// `VSIM key (ELE|FP32|VALUES) ... [WITHSCORES] [WITHATTRIBS] [COUNT num]
/// [EPSILON delta] [EF factor] [FILTER expr] [FILTER-EF max] [TRUTH] [NOTHREAD]`.
/// TRUTH forces an exact linear scan and NOTHREAD keeps it on the main thread, so
/// both must return the same neighbours as the default approximate search.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn vsim_truth_and_no_thread() -> Result<()> {
    let client = get_test_client().await?;

    client.flushall(FlushingMode::Sync).await?;

    for element in ["apple", "apples", "pear"] {
        client
            .vadd(
                "key",
                None,
                &[0.1, 1.2, 0.5],
                element,
                VAddOptions::default(),
            )
            .await?;
    }

    let result: Vec<String> = client
        .vsim(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default().truth().no_thread(),
        )
        .await?;

    assert_eq!(3, result.len());
    assert!(result.contains(&"apple".to_owned()));
    assert!(result.contains(&"apples".to_owned()));
    assert!(result.contains(&"pear".to_owned()));

    Ok(())
}
