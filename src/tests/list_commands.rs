use crate::{
    tests::get_default_addr, ConnectionMultiplexer, GenericCommands, ListCommands, Result,
};
use serial_test::serial;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn lpush_lpop() -> Result<()> {
    let connection = ConnectionMultiplexer::connect(get_default_addr()).await?;
    let database = connection.get_default_database();

    // cleanup
    database.del("mylist").await?;

    let size = database.lpush("mylist", "element1").await?;
    assert_eq!(1, size);

    let size = database.lpush("mylist", ["element2", "element3"]).await?;
    assert_eq!(3, size);

    let elements: Vec<String> = database.lpop("mylist", 2).await?;
    assert_eq!(2, elements.len());
    assert_eq!("element3", elements[0].as_str());
    assert_eq!("element2", elements[1].as_str());

    let elements: Vec<String> = database.lpop("mylist", 1).await?;
    assert_eq!(1, elements.len());
    assert_eq!("element1", elements[0].as_str());

    let elements: Vec<String> = database.lpop("mylist", 1).await?;
    assert_eq!(0, elements.len());

    Ok(())
}
