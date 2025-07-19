use crate::{
    CommandInfoManager, Result,
    client::IntoConfig,
    commands::{
        GenericCommands, MigrateOptions, SortOptions, SortOrder, SortedSetCommands, StreamCommands,
        StringCommands, XReadGroupOptions, XReadOptions, ZAggregate,
    },
    network::StandaloneConnection,
    tests::{get_default_addr, get_default_host, get_default_port, get_test_client},
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn extract_keys() -> Result<()> {
    let client = get_test_client().await?;
    let mut connection = StandaloneConnection::connect(
        &get_default_host(),
        get_default_port(),
        &get_default_addr().into_config()?,
    )
    .await?;
    let command_info_manager = CommandInfoManager::initialize(&mut connection).await?;

    // SET
    let keys = command_info_manager
        .extract_keys(client.set("key", "value").command(), &mut connection)
        .await?;
    assert_eq!(1, keys.len());
    assert_eq!("key", keys[0]);

    // MSET
    let keys = command_info_manager
        .extract_keys(
            client
                .mset([("key1", "value1"), ("key2", "value2")])
                .command(),
            &mut connection,
        )
        .await?;
    assert_eq!(2, keys.len());
    assert_eq!("key1", keys[0]);
    assert_eq!("key2", keys[1]);

    // XREAD
    let keys = command_info_manager
        .extract_keys(
            client
                .xread::<_, _, _, _, String, Vec<(_, _)>>(
                    XReadOptions::default().count(2),
                    ["mystream", "writers"],
                    ["1526999352406-0", "1526985685298-0"],
                )
                .command(),
            &mut connection,
        )
        .await?;
    assert_eq!(2, keys.len());
    assert_eq!("mystream", keys[0]);
    assert_eq!("writers", keys[1]);

    // XREADGROUP
    let keys = command_info_manager
        .extract_keys(
            client
                .xreadgroup::<_, _, _, _, _, _, String, Vec<(_, _)>>(
                    "mygroup",
                    "myconsumer",
                    XReadGroupOptions::default().count(2),
                    ["mystream", "writers"],
                    ["1526999352406-0", "1526985685298-0"],
                )
                .command(),
            &mut connection,
        )
        .await?;
    assert_eq!(2, keys.len(), "unexpected keys: {:?}", keys);
    assert_eq!("mystream", keys[0]);
    assert_eq!("writers", keys[1]);

    // MIGRATE
    let keys = command_info_manager
        .extract_keys(
            client
                .migrate(
                    "192.168.1.34",
                    6379,
                    "",
                    0,
                    5000,
                    MigrateOptions::default().keys(["key1", "key2", "key3"]),
                )
                .command(),
            &mut connection,
        )
        .await?;
    assert_eq!(3, keys.len());
    assert_eq!("key1", keys[0]);
    assert_eq!("key2", keys[1]);
    assert_eq!("key3", keys[2]);

    // ZUNION
    let keys = command_info_manager
        .extract_keys(
            client
                .zunion::<_, _, _, String>(["zset1", "zset2"], Some([1.5, 2.5]), ZAggregate::Max)
                .command(),
            &mut connection,
        )
        .await?;
    assert_eq!(2, keys.len());
    assert_eq!("zset1", keys[0]);
    assert_eq!("zset2", keys[1]);

    // SORT
    let keys = command_info_manager
        .extract_keys(
            client
                .sort_and_store(
                    "src",
                    "dst",
                    SortOptions::default()
                        .limit(0, 5)
                        .alpha()
                        .order(SortOrder::Desc),
                )
                .command(),
            &mut connection,
        )
        .await?;
    assert_eq!(2, keys.len());
    assert_eq!("src", keys[0]);
    assert_eq!("dst", keys[1]);

    Ok(())
}
