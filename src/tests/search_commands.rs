use std::collections::{HashMap, HashSet};

use crate::{
    tests::get_redis_stack_test_client, FlushingMode, FtAggregateOptions, FtCreateOptions,
    FtFieldSchema, FtFieldType, FtIndexDataType, FtLoadAttribute, FtReducer, FtSortBy,
    FtWithCursorOptions, HashCommands, JsonCommands, Result, SearchCommands, ServerCommands,
    SetCondition, SortOrder,
};
use rand::{seq::SliceRandom, Rng};
use serial_test::serial;
use smallvec::SmallVec;

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_aggregate() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "doc:1",
            "$",
            r#"[{"arr": [1, 2, 3]}, {"val": "hello"}, {"val": "world"}]"#,
            SetCondition::None,
        )
        .await?;

    client
        .ft_create(
            "idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Json)
                .prefix("doc"),
            [
                FtFieldSchema::identifier("$..arr")
                    .as_attribute("arr")
                    .field_type(FtFieldType::Numeric),
                FtFieldSchema::identifier("$..val")
                    .as_attribute("val")
                    .field_type(FtFieldType::Text),
            ],
        )
        .await?;

    let _result = client
        .ft_aggregate(
            "idx",
            "*",
            FtAggregateOptions::default()
                .load([FtLoadAttribute::new("arr"), FtLoadAttribute::new("val")]),
        )
        .await?;

    let _result = client
        .ft_aggregate(
            "idx1",
            r#"@url:"about.html""#,
            FtAggregateOptions::default()
                .apply("day(@timestamp)", "day")
                .groupby(
                    ["@day", "@country"],
                    FtReducer::count().as_name("num_visits"),
                )
                .sortby(FtSortBy::property("@day"), None),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "books-idx",
            "*",
            FtAggregateOptions::default()
                .groupby(
                    "@published_year",
                    FtReducer::count().as_name("num_published"),
                )
                .groupby(
                    Vec::<String>::new(),
                    FtReducer::max("@num_published").as_name("max_books_published_per_year"),
                ),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "libraries-idx",
            "@location:[-73.982254 40.753181 10 km]",
            FtAggregateOptions::default()
                .load(FtLoadAttribute::new("@location"))
                .apply("geodistance(@location, -73.982254, 40.753181)", "day"),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "gh",
            "*",
            FtAggregateOptions::default()
                .groupby("@actor", FtReducer::count().as_name("num"))
                .sortby(FtSortBy::property("@day").desc(), Some(10)),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "idx2",
            "*",
            FtAggregateOptions::default().withcursor(FtWithCursorOptions::default().count(10)),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "idx2",
            "*",
            FtAggregateOptions::default().withcursor(FtWithCursorOptions::default().maxidle(10000)),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "idx2",
            "*",
            FtAggregateOptions::default().groupby(
                Vec::<String>::new(),
                FtReducer::first_value_by_order("@name", "@age", SortOrder::Desc),
            ),
        )
        .await;

    // example from Redis official documentation
    // https://redis.io/docs/stack/search/reference/aggregations/#quick-example
    client
        .hset(
            "log:1",
            [
                ("url", "page1.html".to_owned()),
                ("timestamp", 1668637156.to_string()),
                ("country", "fr".to_owned()),
                ("user_id", "john".to_owned()),
            ],
        )
        .await?;

    client
        .hset(
            "log:2",
            [
                ("url", "page2.html".to_owned()),
                ("timestamp", 1668637157.to_string()),
                ("country", "fr".to_owned()),
                ("user_id", "bill".to_owned()),
            ],
        )
        .await?;

    client
        .hset(
            "log:3",
            [
                ("url", "page3.html".to_owned()),
                ("timestamp", 1668657158.to_string()),
                ("country", "ca".to_owned()),
                ("user_id", "tom".to_owned()),
            ],
        )
        .await?;

    client
        .hset(
            "log:4",
            [
                ("url", "page4.html".to_owned()),
                ("timestamp", 1668657159.to_string()),
                ("country", "ca".to_owned()),
                ("user_id", "mike".to_owned()),
            ],
        )
        .await?;

    client
        .ft_create(
            "myIndex",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("log"),
            [
                FtFieldSchema::identifier("url")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("timestamp")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
                FtFieldSchema::identifier("country")
                    .field_type(FtFieldType::Tag)
                    .sortable(),
                FtFieldSchema::identifier("user_id")
                    .field_type(FtFieldType::Text)
                    .noindex()
                    .sortable(),
            ],
        )
        .await?;

    let result = client
        .ft_aggregate(
            "myIndex",
            "*",
            FtAggregateOptions::default()
                .apply("@timestamp - (@timestamp % 3600)", "hour")
                .groupby(
                    "@hour",
                    FtReducer::count_distinct("@user_id").as_name("num_users"),
                )
                .sortby(FtSortBy::property("@hour").asc(), None)
                .apply("timefmt(@hour)", "hour"),
        )
        .await?;

    assert_eq!(None, result.cursor_id);
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());
    assert_eq!(2, result.results[0].len());
    assert_eq!(2, result.results[1].len());
    assert_eq!(
        ("hour".to_owned(), "2022-11-16T22:00:00Z".to_owned()),
        result.results[0][0]
    );
    assert_eq!(
        ("num_users".to_owned(), "2".to_owned()),
        result.results[0][1]
    );
    assert_eq!(
        ("hour".to_owned(), "2022-11-17T03:00:00Z".to_owned()),
        result.results[1][0]
    );
    assert_eq!(
        ("num_users".to_owned(), "2".to_owned()),
        result.results[1][1]
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_alias() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;

    client.ft_aliasadd("alias", "idx1").await?;
    client.ft_aliasupdate("alias", "idx2").await?;
    client.ft_aliasdel("alias").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_alter() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field1").field_type(FtFieldType::Text),
        )
        .await?;

    client
        .ft_alter(
            "idx1",
            false,
            FtFieldSchema::identifier("field2").field_type(FtFieldType::Text),
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_config_get_set() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.ft_config_set("TIMEOUT", 42).await?;

    let result: SmallVec<[(String, u64); 1]> = client.ft_config_get("TIMEOUT").await?;
    assert_eq!(("TIMEOUT".to_owned(), 42), result[0]);

    let result: HashMap<String, String> = client.ft_config_get("*").await?;
    assert!(!result.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_create() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default().on(FtIndexDataType::Hash),
            [
                FtFieldSchema::identifier("title")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("published_at")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
                FtFieldSchema::identifier("category")
                    .field_type(FtFieldType::Tag)
                    .sortable(),
            ],
        )
        .await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("blog:post"),
            [
                FtFieldSchema::identifier("sku")
                    .as_attribute("sku_text")
                    .field_type(FtFieldType::Text),
                FtFieldSchema::identifier("sku")
                    .as_attribute("sku_tag")
                    .field_type(FtFieldType::Tag)
                    .sortable(),
            ],
        )
        .await?;

    client
        .ft_create(
            "author-books-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix(["author:details:", "book:details:"]),
            [
                FtFieldSchema::identifier("author_id").field_type(FtFieldType::Tag),
                FtFieldSchema::identifier("title").field_type(FtFieldType::Text),
                FtFieldSchema::identifier("name").field_type(FtFieldType::Text),
            ],
        )
        .await?;

    client
        .ft_create(
            "g-authors-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("author:details")
                .filter(r#"startswith(@name, "G")"#),
            FtFieldSchema::identifier("name").field_type(FtFieldType::Text),
        )
        .await?;

    client
        .ft_create(
            "subtitled-books-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("book:details")
                .filter(r#"@subtitle != """#),
            FtFieldSchema::identifier("title").field_type(FtFieldType::Text),
        )
        .await?;

    client
        .ft_create(
            "books-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("book:details"),
            [
                FtFieldSchema::identifier("title").field_type(FtFieldType::Text),
                FtFieldSchema::identifier("categories")
                    .field_type(FtFieldType::Tag)
                    .separator(';'),
            ],
        )
        .await?;

    client
        .ft_create(
            "idx3",
            FtCreateOptions::default()
                .on(FtIndexDataType::Json)
                .prefix("book:details"),
            [
                FtFieldSchema::identifier("$.title")
                    .as_attribute("title")
                    .field_type(FtFieldType::Text),
                FtFieldSchema::identifier("$.categories")
                    .as_attribute("categories")
                    .field_type(FtFieldType::Tag),
            ],
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_cursor() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    for i in 0..100 {
        client
            .hset(
                format!("log:{i}"),
                [
                    (
                        "url",
                        format!("page{}.html", rand::thread_rng().gen_range(1..21)).to_owned(),
                    ),
                    ("timestamp", (1668637156 + i).to_string()),
                    (
                        "country",
                        (*["fr", "ca"].choose(&mut rand::thread_rng()).unwrap()).to_owned(),
                    ),
                    (
                        "user_id",
                        format!("user{}", rand::thread_rng().gen_range(1..11)),
                    ),
                ],
            )
            .await?;
    }

    client
        .ft_create(
            "myIndex",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("log"),
            [
                FtFieldSchema::identifier("url")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("timestamp")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
                FtFieldSchema::identifier("country")
                    .field_type(FtFieldType::Tag)
                    .sortable(),
                FtFieldSchema::identifier("user_id")
                    .field_type(FtFieldType::Text)
                    .noindex()
                    .sortable(),
            ],
        )
        .await?;

    let result = client
        .ft_aggregate(
            "myIndex",
            "*",
            FtAggregateOptions::default()
                .groupby(
                    "@url",
                    FtReducer::count_distinct("@user_id").as_name("num_users"),
                )
                .sortby(FtSortBy::property("@num_users").desc(), None)
                .limit(0, 100)
                .withcursor(FtWithCursorOptions::default().count(10)),
        )
        .await?;

    assert!(result.cursor_id.is_some());
    assert_eq!(20, result.total_results);
    assert_eq!(10, result.results.len());

    let result = client
        .ft_cursor_read("myIndex", result.cursor_id.unwrap())
        .await?;

    assert!(result.cursor_id.is_some());
    assert_eq!(10, result.results.len());

    client
        .ft_cursor_del("myIndex", result.cursor_id.unwrap())
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_dict() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let num = client
        .ft_dictadd("dict", ["term1", "term2", "term3"])
        .await?;
    assert_eq!(3, num);

    let num = client.ft_dictdel("dict", ["term1", "term3"]).await?;
    assert_eq!(2, num);

    let num = client.ft_dictadd("dict", "term4").await?;
    assert_eq!(1, num);

    let num = client.ft_dictdel("dict", "term1").await?;
    assert_eq!(0, num);

    let terms: HashSet<String> = client.ft_dictdump("dict").await?;
    assert!(terms.contains("term2"));
    assert!(terms.contains("term4"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_dropindex() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.ft_dropindex("myIndex", false).await;
    assert!(result.is_err());

    client
        .hset(
            "log:1",
            [
                ("url", "page1.html".to_owned()),
                ("timestamp", 1668637156.to_string()),
                ("country", "fr".to_owned()),
                ("user_id", "john".to_owned()),
            ],
        )
        .await?;

    client
        .ft_create(
            "myIndex",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("log"),
            [
                FtFieldSchema::identifier("url")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("timestamp")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
                FtFieldSchema::identifier("country")
                    .field_type(FtFieldType::Tag)
                    .sortable(),
                FtFieldSchema::identifier("user_id")
                    .field_type(FtFieldType::Text)
                    .noindex()
                    .sortable(),
            ],
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_list() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;

    client
        .ft_create(
            "idx3",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;

    let index_names: Vec<String> = client.ft_list().await?;
    assert_eq!(3, index_names.len());
    assert!(index_names.contains(&"idx1".to_owned()));
    assert!(index_names.contains(&"idx2".to_owned()));
    assert!(index_names.contains(&"idx3".to_owned()));

    Ok(())
}
