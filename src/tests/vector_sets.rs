use std::{collections::HashSet, hash::Hash};

use crate::{
    commands::{
        FlushingMode, ServerCommands, VAddOptions, VSimOptions, VectorOrElement, VectorSetCommands,
    },
    tests::get_test_client,
    Result,
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
    assert_eq!(1, result.len());
    assert_eq!(None, result[0]);

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
        .vsim_with_scores(
            "key",
            VectorOrElement::Element("apple"),
            VSimOptions::default(),
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
