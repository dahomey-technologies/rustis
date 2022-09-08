use std::collections::HashSet;

use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, GeoCommands, GeoSearchBy,
    GeoSearchFrom, GeoSearchOrder, GeoSearchResult, GeoUnit, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geoadd() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("key").await?;

    let len = database
        .geoadd("key")
        .execute([(1.0, 1.0, "location1"), (2.0, 2.0, "location2")])
        .await?;
    assert_eq!(2, len);

    let len = database
        .geoadd("key")
        .execute([(1.0, 1.0, "location1"), (2.0, 2.0, "location2")])
        .await?;
    assert_eq!(0, len);

    let len = database
        .geoadd("key")
        .ch()
        .execute([(2.0, 2.0, "location1"), (2.0, 2.0, "location2")])
        .await?;
    assert_eq!(1, len);

    let len = database
        .geoadd("key")
        .ch()
        .xx()
        .execute([
            (1.0, 1.0, "location1"),
            (2.0, 2.0, "location2"),
            (3.0, 3.0, "location3"),
        ])
        .await?;
    assert_eq!(1, len);

    let len = database
        .geoadd("key")
        .ch()
        .nx()
        .execute([
            (2.0, 2.0, "location1"),
            (2.0, 2.0, "location2"),
            (3.0, 3.0, "location3"),
        ])
        .await?;
    assert_eq!(1, len);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geodist() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("Sicily").await?;

    let len = database
        .geoadd("Sicily")
        .execute([
            (13.361389, 38.115556, "Palermo"),
            (15.087269, 37.502669, "Catania"),
        ])
        .await?;
    assert_eq!(2, len);

    let dist = database
        .geodist("Sicily", "Palermo", "Catania", GeoUnit::Meters)
        .await?;
    assert_eq!(Some(166274.1516), dist);

    let dist = database
        .geodist("Sicily", "Palermo", "Catania", GeoUnit::Kilometers)
        .await?;
    assert_eq!(Some(166.2742), dist);

    let dist = database
        .geodist("Sicily", "Palermo", "Catania", GeoUnit::Miles)
        .await?;
    assert_eq!(Some(103.3182), dist);

    let dist = database
        .geodist("Sicily", "Foo", "Bar", GeoUnit::Meters)
        .await?;
    assert_eq!(None, dist);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geohash() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("Sicily").await?;

    let len = database
        .geoadd("Sicily")
        .execute([
            (13.361389, 38.115556, "Palermo"),
            (15.087269, 37.502669, "Catania"),
        ])
        .await?;
    assert_eq!(2, len);

    let hashes = database.geohash("Sicily", ["Palermo", "Catania"]).await?;
    assert_eq!(2, hashes.len());
    assert_eq!("sqc8b49rny0", hashes[0]);
    assert_eq!("sqdtr74hyu0", hashes[1]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn geopos() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("Sicily").await?;

    let len = database
        .geoadd("Sicily")
        .execute([
            (13.361389, 38.115556, "Palermo"),
            (15.087269, 37.502669, "Catania"),
        ])
        .await?;
    assert_eq!(2, len);

    let hashes = database
        .geopos("Sicily", ["Palermo", "Catania", "NonExisting"])
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("Sicily").await?;

    let len = database
        .geoadd("Sicily")
        .execute([
            (13.361389, 38.115556, "Palermo"),
            (15.087269, 37.502669, "Catania"),
        ])
        .await?;
    assert_eq!(2, len);

    let len = database
        .geoadd("Sicily")
        .execute([
            (12.758489, 38.788135, "edge1"),
            (17.241510, 38.788135, "edge2"),
        ])
        .await?;
    assert_eq!(2, len);

    let results: HashSet<String> = database
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
        )
        .execute()
        .await?;
    assert_eq!(2, results.len());
    assert!(results.contains("Palermo"));
    assert!(results.contains("Catania"));

    let results: Vec<GeoSearchResult<String>> = database
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
        )
        .order(GeoSearchOrder::Asc)
        .with_coord()
        .with_dist()
        .execute()
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
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del(["Sicily", "out"]).await?;

    let len = database
        .geoadd("Sicily")
        .execute([
            (13.361389, 38.115556, "Palermo"),
            (15.087269, 37.502669, "Catania"),
        ])
        .await?;
    assert_eq!(2, len);

    let len = database
        .geoadd("Sicily")
        .execute([
            (12.758489, 38.788135, "edge1"),
            (17.241510, 38.788135, "edge2"),
        ])
        .await?;
    assert_eq!(2, len);

    let len = database
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
        )
        .order(GeoSearchOrder::Asc)
        .count(3, false)
        .execute()
        .await?;
    assert_eq!(3, len);

    let results: Vec<GeoSearchResult<String>> = database
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
        )
        .order(GeoSearchOrder::Asc)
        .with_coord()
        .with_dist()
        .with_hash()
        .execute()
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
