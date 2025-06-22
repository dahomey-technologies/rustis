use crate::{
    commands::{FlushingMode, ServerCommands, TDigestCommands, TDigestMergeOptions},
    tests::get_test_client,
    Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_add() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.tdigest_add("key", [1., 2., 3.]).await;
    assert!(result.is_err()); // key does not exist

    client.tdigest_create("key", Some(100)).await?;
    client.tdigest_add("key", [1., 2., 3.]).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_create() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(100)).await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_byrank() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let values: Vec<f64> = client
        .tdigest_byrank(
            "key",
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        )
        .await?;
    assert_eq!(
        vec![
            1.0,
            2.0,
            2.0,
            3.0,
            3.0,
            3.0,
            4.0,
            4.0,
            4.0,
            4.0,
            5.0,
            5.0,
            5.0,
            5.0,
            5.0,
            f64::INFINITY
        ],
        values
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_byrevrank() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let values: Vec<f64> = client
        .tdigest_byrevrank(
            "key",
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        )
        .await?;
    assert_eq!(
        vec![
            5.0,
            5.0,
            5.0,
            5.0,
            5.0,
            4.0,
            4.0,
            4.0,
            4.0,
            3.0,
            3.0,
            3.0,
            2.0,
            2.0,
            1.0,
            f64::NEG_INFINITY
        ],
        values
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_cdf() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let values: Vec<f64> = client.tdigest_cdf("key", [0, 1, 2, 3, 4, 5, 6]).await?;
    assert_eq!(
        vec![
            0.,
            0.033_333_333_333_333_33,
            0.133_333_333_333_333_33,
            0.3,
            0.533_333_333_333_333_3,
            0.833_333_333_333_333_4,
            1.
        ],
        values
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_info() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let info = client.tdigest_info("key").await?;
    log::debug!("info: {info:?}");
    assert_eq!(15, info.observations);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_max() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    let max = client.tdigest_max("key").await?;
    assert!(max.is_nan());

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let max = client.tdigest_max("key").await?;
    assert_eq!(5., max);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_merge() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("s1", None).await?;
    client.tdigest_create("s2", None).await?;

    client.tdigest_add("s1", [10., 20.]).await?;
    client.tdigest_add("s2", [30., 40.]).await?;

    client
        .tdigest_merge("sM", ["s1", "s2"], TDigestMergeOptions::default())
        .await?;

    let results: Vec<f64> = client.tdigest_byrank("sM", [0, 1, 2, 3, 4]).await?;
    assert_eq!(vec![10., 20., 30., 40., f64::INFINITY], results);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_min() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    let max = client.tdigest_min("key").await?;
    assert!(max.is_nan());

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let max = client.tdigest_min("key").await?;
    assert_eq!(1., max);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_quantile() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add(
            "key",
            [1., 2., 2., 3., 3., 3., 4., 4., 4., 4., 5., 5., 5., 5., 5.],
        )
        .await?;

    let values: Vec<f64> = client
        .tdigest_quantile("key", [0., 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.])
        .await?;
    assert_eq!(vec![1., 2., 3., 3., 4., 4., 4., 5., 5., 5., 5.], values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_rank_revrank() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add("key", [10., 20., 30., 40., 50., 60.])
        .await?;

    let values: Vec<isize> = client
        .tdigest_rank("key", [0, 10, 20, 30, 40, 50, 60, 70])
        .await?;
    assert_eq!(vec![-1, 0, 1, 2, 3, 4, 5, 6], values);

    let values: Vec<isize> = client
        .tdigest_revrank("key", [0, 10, 20, 30, 40, 50, 60, 70])
        .await?;
    assert_eq!(vec![6, 5, 4, 3, 2, 1, 0, -1], values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_reset() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add("key", [10., 20., 30., 40., 50., 60.])
        .await?;

    let values: Vec<isize> = client.tdigest_rank("key", [10, 20, 30, 40, 50, 60]).await?;
    assert_eq!(vec![0, 1, 2, 3, 4, 5], values);

    client.tdigest_reset("key").await?;

    let values: Vec<isize> = client.tdigest_rank("key", [10, 20, 30, 40, 50, 60]).await?;
    assert_eq!(vec![-2, -2, -2, -2, -2, -2], values);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn tdigest_trimmed_mean() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.tdigest_create("key", Some(1000)).await?;

    client
        .tdigest_add("key", [1., 2., 3., 4., 5., 6., 7., 8., 9., 10.])
        .await?;

    let mean_value = client.tdigest_trimmed_mean("key", 0.1, 0.6).await?;
    assert_eq!(4., mean_value);

    let mean_value = client.tdigest_trimmed_mean("key", 0.3, 0.9).await?;
    assert_eq!(6.5, mean_value);

    let mean_value = client.tdigest_trimmed_mean("key", 0., 1.).await?;
    assert_eq!(5.5, mean_value);

    Ok(())
}
