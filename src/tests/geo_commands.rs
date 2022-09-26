use crate::{
    tests::get_test_client, ClientCommandResult, GenericCommands, GeoAddCondition, GeoCommands,
    GeoSearchBy, GeoSearchFrom, GeoSearchOptions, GeoSearchOrder, GeoSearchResult,
    GeoSearchStoreOptions, GeoUnit, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geoadd() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("key").send().await?;

    let len = client
        .geoadd(
            "key",
            Default::default(),
            false,
            [(1.0, 1.0, "location1"), (2.0, 2.0, "location2")],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let len = client
        .geoadd(
            "key",
            Default::default(),
            false,
            [(1.0, 1.0, "location1"), (2.0, 2.0, "location2")],
        )
        .send()
        .await?;
    assert_eq!(0, len);

    let len = client
        .geoadd(
            "key",
            Default::default(),
            true,
            [(2.0, 2.0, "location1"), (2.0, 2.0, "location2")],
        )
        .send()
        .await?;
    assert_eq!(1, len);

    let len = client
        .geoadd(
            "key",
            GeoAddCondition::XX,
            true,
            [
                (1.0, 1.0, "location1"),
                (2.0, 2.0, "location2"),
                (3.0, 3.0, "location3"),
            ],
        )
        .send()
        .await?;
    assert_eq!(1, len);

    let len = client
        .geoadd(
            "key",
            GeoAddCondition::NX,
            true,
            [
                (2.0, 2.0, "location1"),
                (2.0, 2.0, "location2"),
                (3.0, 3.0, "location3"),
            ],
        )
        .send()
        .await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geodist() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("Sicily").send().await?;

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (13.361389, 38.115556, "Palermo"),
                (15.087269, 37.502669, "Catania"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let dist = client
        .geodist("Sicily", "Palermo", "Catania", GeoUnit::Meters)
        .send()
        .await?;
    assert_eq!(Some(166274.1516), dist);

    let dist = client
        .geodist("Sicily", "Palermo", "Catania", GeoUnit::Kilometers)
        .send()
        .await?;
    assert_eq!(Some(166.2742), dist);

    let dist = client
        .geodist("Sicily", "Palermo", "Catania", GeoUnit::Miles)
        .send()
        .await?;
    assert_eq!(Some(103.3182), dist);

    let dist = client
        .geodist("Sicily", "Foo", "Bar", GeoUnit::Meters)
        .send()
        .await?;
    assert_eq!(None, dist);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geohash() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("Sicily").send().await?;

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (13.361389, 38.115556, "Palermo"),
                (15.087269, 37.502669, "Catania"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let hashes = client
        .geohash("Sicily", ["Palermo", "Catania"])
        .send()
        .await?;
    assert_eq!(2, hashes.len());
    assert_eq!("sqc8b49rny0", hashes[0]);
    assert_eq!("sqdtr74hyu0", hashes[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geopos() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("Sicily").send().await?;

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (13.361389, 38.115556, "Palermo"),
                (15.087269, 37.502669, "Catania"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let hashes = client
        .geopos("Sicily", ["Palermo", "Catania", "NonExisting"])
        .send()
        .await?;
    assert_eq!(3, hashes.len());
    assert_eq!(
        Some((13.36138933897018433, 38.11555639549629859)),
        hashes[0]
    );
    assert_eq!(
        Some((15.08726745843887329, 37.50266842333162032)),
        hashes[1]
    );
    assert_eq!(None, hashes[2]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geosearch() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del("Sicily").send().await?;

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (13.361389, 38.115556, "Palermo"),
                (15.087269, 37.502669, "Catania"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (12.758489, 38.788135, "edge1"),
                (17.241510, 38.788135, "edge2"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let results: Vec<GeoSearchResult<String>> = client
        .geosearch(
            "Sicily",
            GeoSearchFrom::FromLonLat::<String> {
                longitude: 15.0,
                latitude: 37.0,
            },
            GeoSearchBy::ByRadius {
                radius: 200.0,
                unit: GeoUnit::Kilometers,
            },
            GeoSearchOptions::default(),
        )
        .send()
        .await?;
    assert_eq!(2, results.len());
    assert!(results.iter().any(|r| r.member == "Palermo",));
    assert!(results.iter().any(|r| r.member == "Catania"));

    let results: Vec<GeoSearchResult<String>> = client
        .geosearch(
            "Sicily",
            GeoSearchFrom::FromLonLat::<String> {
                longitude: 15.0,
                latitude: 37.0,
            },
            GeoSearchBy::ByBox {
                width: 400.0,
                height: 400.0,
                unit: GeoUnit::Kilometers,
            },
            GeoSearchOptions::default()
                .order(GeoSearchOrder::Asc)
                .with_coord()
                .with_dist(),
        )
        .send()
        .await?;

    assert_eq!(4, results.len());
    assert_eq!("Catania", results[0].member);
    assert_eq!(Some(56.4413), results[0].distance);
    assert_eq!(None, results[0].geo_hash);
    assert_eq!(
        Some((15.087267458438873, 37.50266842333162)),
        results[0].coordinates
    );
    assert_eq!("Palermo", results[1].member);
    assert_eq!(Some(190.4424), results[1].distance);
    assert_eq!(None, results[1].geo_hash);
    assert_eq!(
        Some((13.36138933897018433, 38.11555639549629859)),
        results[1].coordinates
    );
    assert_eq!("edge2", results[2].member);
    assert_eq!(Some(279.7403), results[2].distance);
    assert_eq!(None, results[2].geo_hash);
    assert_eq!(
        Some((17.24151045083999634, 38.78813451624225195)),
        results[2].coordinates
    );
    assert_eq!("edge1", results[3].member);
    assert_eq!(Some(279.7405), results[3].distance);
    assert_eq!(None, results[3].geo_hash);
    assert_eq!(
        Some((12.7584877610206604, 38.78813451624225195)),
        results[3].coordinates
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geosearchstore() -> Result<()> {
    let client = get_test_client().await?;

    // cleanup
    client.del(["Sicily", "out"]).send().await?;

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (13.361389, 38.115556, "Palermo"),
                (15.087269, 37.502669, "Catania"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let len = client
        .geoadd(
            "Sicily",
            Default::default(),
            false,
            [
                (12.758489, 38.788135, "edge1"),
                (17.241510, 38.788135, "edge2"),
            ],
        )
        .send()
        .await?;
    assert_eq!(2, len);

    let len = client
        .geosearchstore(
            "out",
            "Sicily",
            GeoSearchFrom::FromLonLat::<String> {
                longitude: 15.0,
                latitude: 37.0,
            },
            GeoSearchBy::ByBox {
                width: 400.0,
                height: 400.0,
                unit: GeoUnit::Kilometers,
            },
            GeoSearchStoreOptions::default()
                .order(GeoSearchOrder::Asc)
                .count(3, false),
        )
        .send()
        .await?;
    assert_eq!(3, len);

    let results: Vec<GeoSearchResult<String>> = client
        .geosearch(
            "out",
            GeoSearchFrom::FromLonLat::<String> {
                longitude: 15.0,
                latitude: 37.0,
            },
            GeoSearchBy::ByBox {
                width: 400.0,
                height: 400.0,
                unit: GeoUnit::Kilometers,
            },
            GeoSearchOptions::default()
                .order(GeoSearchOrder::Asc)
                .with_coord()
                .with_dist()
                .with_hash(),
        )
        .send()
        .await?;

    assert_eq!(3, results.len());
    assert_eq!("Catania", results[0].member);
    assert_eq!(Some(56.4413), results[0].distance);
    assert_eq!(Some(3479447370796909), results[0].geo_hash);
    assert_eq!(
        Some((15.087267458438873, 37.50266842333162)),
        results[0].coordinates
    );
    assert_eq!("Palermo", results[1].member);
    assert_eq!(Some(190.4424), results[1].distance);
    assert_eq!(Some(3479099956230698), results[1].geo_hash);
    assert_eq!(
        Some((13.36138933897018433, 38.11555639549629859)),
        results[1].coordinates
    );
    assert_eq!("edge2", results[2].member);
    assert_eq!(Some(279.7403), results[2].distance);
    assert_eq!(Some(3481342659049484), results[2].geo_hash);
    assert_eq!(
        Some((17.24151045083999634, 38.78813451624225195)),
        results[2].coordinates
    );

    Ok(())
}
