use crate::{
    client::{BatchPreparedCommand, Client},
    commands::{
        ClientReplyMode, ConnectionCommands, FlushingMode, FtAggregateOptions, FtCreateOptions,
        FtFieldSchema, FtFieldType, FtFlatVectorFieldAttributes, FtIndexDataType, FtLanguage,
        FtLoadAttribute, FtReducer, FtSearchOptions, FtSearchResult, FtSortBy, FtSpellCheckOptions,
        FtSugAddOptions, FtSugGetOptions, FtTermType, FtVectorDistanceMetric,
        FtVectorFieldAlgorithm, FtVectorType, FtWithCursorOptions, HashCommands, JsonCommands,
        SearchCommands, ServerCommands, SetCondition, SortOrder,
    },
    network::sleep,
    tests::get_redis_stack_test_client,
    Result,
};
use rand::{seq::SliceRandom, Rng};
use serial_test::serial;
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

async fn wait_for_index_scanned(client: &Client, index: &str) -> Result<()> {
    loop {
        let result = client.ft_info(index.to_owned()).await?;

        if !result.indexing {
            break;
        }

        sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_aggregate() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
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
    wait_for_index_scanned(&client, "idx").await?;

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
            "index",
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

    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_aggregate(
            "index",
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
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;
    wait_for_index_scanned(&client, "idx1").await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("field").field_type(FtFieldType::Text),
        )
        .await?;
    wait_for_index_scanned(&client, "idx2").await?;

    client.ft_aliasadd("alias", "idx1").await?;
    client.ft_aliasupdate("alias", "idx2").await?;
    client.ft_aliasdel("alias").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_alter() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
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
    let client = get_redis_stack_test_client().await?;
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
    let client = get_redis_stack_test_client().await?;
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

    // vector
    // See: https://redis.io/docs/interact/search-and-query/search/vectors/#making-the-bikes-collection-searchable
    client
        .ft_create(
            "idx:bikes_vss",
            FtCreateOptions::default()
                .on(FtIndexDataType::Json)
                .prefix("bikes:")
                .score(1.0),
            [
                FtFieldSchema::identifier("$.model")
                    .field_type(FtFieldType::Text)
                    .weight(1.0)
                    .nostem(),
                FtFieldSchema::identifier("$.brand")
                    .field_type(FtFieldType::Text)
                    .weight(1.0)
                    .nostem(),
                FtFieldSchema::identifier("$.price").field_type(FtFieldType::Numeric),
                FtFieldSchema::identifier("$.type")
                    .field_type(FtFieldType::Tag)
                    .separator(','),
                FtFieldSchema::identifier("$.description")
                    .as_attribute("description")
                    .field_type(FtFieldType::Text)
                    .weight(1.0)
                    .nostem(),
                FtFieldSchema::identifier("$.description_embeddings ").field_type(
                    FtFieldType::Vector(Some(FtVectorFieldAlgorithm::Flat(
                        FtFlatVectorFieldAttributes::new(
                            FtVectorType::Float32,
                            768,
                            FtVectorDistanceMetric::Cosine,
                        ),
                    ))),
                ),
            ],
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_cursor() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();

    pipeline.client_reply(ClientReplyMode::Off).forget();

    for i in 1..1001 {
        pipeline
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
            .forget();
    }

    pipeline.client_reply(ClientReplyMode::On).forget();

    pipeline.execute().await?;

    client
        .ft_create(
            "index",
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
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_aggregate(
            "index",
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
        .ft_cursor_read("index", result.cursor_id.unwrap())
        .await?;

    assert!(result.cursor_id.is_some());
    assert_eq!(10, result.results.len());

    client
        .ft_cursor_del("index", result.cursor_id.unwrap())
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_dict() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
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
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.ft_dropindex("index", false).await;
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
            "index",
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
    wait_for_index_scanned(&client, "index").await?;

    client.ft_dropindex("index", false).await?;
    let exists = client.hexists("log:1", "url").await?;
    assert!(exists);

    client
        .ft_create(
            "index",
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
    wait_for_index_scanned(&client, "index").await?;

    client.ft_dropindex("index", true).await?;

    let exists = client.hexists("log:1", "url").await?;
    assert!(!exists);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_explain() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default(),
            [
                FtFieldSchema::identifier("text")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("date")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
            ],
        )
        .await?;

    let execution_plan: String = client
        .ft_explain(
            "index",
            "(foo bar)|(hello world) @date:[100 200]|@date:[500 +inf]",
            None,
        )
        .await?;
    assert!(!execution_plan.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_explaincli() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default(),
            [
                FtFieldSchema::identifier("text")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("date")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
            ],
        )
        .await?;

    let execution_plan: Vec<String> = client
        .ft_explaincli(
            "index",
            "(foo bar)|(hello world) @date:[100 200]|@date:[500 +inf]",
            None,
        )
        .await?;
    assert!(!execution_plan.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_info() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .filter(r#"@indexName=="myindexname""#)
                .language(FtLanguage::French)
                .language_field("language")
                .score(0.5)
                .score_field("score")
                .payload_field("payload")
                .max_text_fields()
                .temporary(500)
                .nohl()
                .nofields()
                .nofreqs()
                .prefix(["log", "doc"])
                .skip_initial_scan()
                .stop_words(["hello", "world"]),
            [
                FtFieldSchema::identifier("text")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("date")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
            ],
        )
        .await?;

    let info = client.ft_info("index").await?;
    log::debug!("info: {info:?}");

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_list() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
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

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_profile() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let mut pipeline = client.create_pipeline();

    pipeline.client_reply(ClientReplyMode::Off).forget();

    for i in 1..1001 {
        pipeline
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
            .forget();
    }

    pipeline.client_reply(ClientReplyMode::On).forget();

    pipeline.execute().await?;

    client
        .ft_create(
            "index",
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
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_profile_aggregate(
            "index",
            false,
            [
                "*",
                "groupby",
                "1",
                "@url",
                "reduce",
                "count_distinct",
                "1",
                "@user_id",
                "as",
                "num_users",
                "sortby",
                "2",
                "@num_users",
                "desc",
                "limit",
                "0",
                "100",
            ],
        )
        .await?;

    log::debug!("result: {result:?}");

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_search() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .hset(
            "doc:1",
            [
                ("title", "dogs"),
                ("data", "foo wizard bar"),
                ("published_at", "2019"),
                ("payload", "tag1"),
            ],
        )
        .await?;
    client
        .hset(
            "doc:2",
            [
                ("title", "cats"),
                ("data", "hello world wizard"),
                ("published_at", "2020"),
                ("payload", "tag2"),
            ],
        )
        .await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("doc")
                .payload_field("payload"),
            [
                FtFieldSchema::identifier("title")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("data")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("published_at")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
            ],
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_search("index", "wizard", FtSearchOptions::default())
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    let result = client
        .ft_search("index", "@title:dogs", FtSearchOptions::default())
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(1, result.total_results);
    assert_eq!(1, result.results.len());

    let result = client
        .ft_search(
            "index",
            "@published_at:[2020 2021]",
            FtSearchOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(1, result.total_results);
    assert_eq!(1, result.results.len());

    let result = client
        .ft_search("index", "*", FtSearchOptions::default().nocontent())
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    let result = client
        .ft_search(
            "index",
            "*",
            FtSearchOptions::default()
                .withscores()
                .withsortkeys()
                .withpayloads()
                .sortby("title", SortOrder::Asc),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    // with pipeline
    let mut pipeline = client.create_pipeline();
    pipeline
        .ft_search("index", "wizard", FtSearchOptions::default())
        .queue();
    let result: FtSearchResult = pipeline.execute().await?;
    log::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_search_empty_index() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("doc")
                .payload_field("payload"),
            [
                FtFieldSchema::identifier("title")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("data")
                    .field_type(FtFieldType::Text)
                    .sortable(),
                FtFieldSchema::identifier("published_at")
                    .field_type(FtFieldType::Numeric)
                    .sortable(),
            ],
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_search("index", "wizard", FtSearchOptions::default())
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(0, result.total_results);
    assert_eq!(0, result.results.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_spellcheck() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.hset("doc", ("text", "hello help")).await?;
    client
        .ft_create(
            "index",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("text").field_type(FtFieldType::Text),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_spellcheck("index", "held", FtSpellCheckOptions::default().distance(2))
        .await?;

    assert_eq!(1, result.misspelled_terms.len());
    assert_eq!("held", result.misspelled_terms[0].misspelled_term);
    assert_eq!(2, result.misspelled_terms[0].suggestions.len());
    assert!(result.misspelled_terms[0]
        .suggestions
        .iter()
        .any(|(_score, suggestion)| suggestion == "hello"));
    assert!(result.misspelled_terms[0]
        .suggestions
        .iter()
        .any(|(_score, suggestion)| suggestion == "help"));

    client.ft_dictadd("dict", "store").await?;

    let result = client
        .ft_spellcheck(
            "index",
            "held|stor",
            FtSpellCheckOptions::default().terms(FtTermType::Include, "dict"),
        )
        .await?;

    assert_eq!(2, result.misspelled_terms.len());
    assert_eq!("held", result.misspelled_terms[0].misspelled_term);
    assert_eq!(1, result.misspelled_terms[0].suggestions.len());
    assert_eq!("help", result.misspelled_terms[0].suggestions[0].1);
    assert_eq!("stor", result.misspelled_terms[1].misspelled_term);
    assert_eq!(1, result.misspelled_terms[1].suggestions.len());
    assert_eq!("store", result.misspelled_terms[1].suggestions[0].1);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_syn() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // Insert documents
    client.hset("foo", ("t", "hello")).await?;
    client.hset("bar", ("t", "world")).await?;

    // Create an index
    client
        .ft_create(
            "index",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("t").field_type(FtFieldType::Text),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    // search => only foo is matched
    let result = client
        .ft_search("index", "hello", FtSearchOptions::default())
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(1, result.total_results);
    assert_eq!(1, result.results.len());
    assert_eq!("foo", result.results[0].document_id);
    assert_eq!(
        ("t".to_owned(), "hello".to_owned()),
        result.results[0].values[0]
    );

    // Create a synonym group
    client
        .ft_synupdate("index", "group1", false, ["hello", "world"])
        .await?;
    let result: HashMap<String, Vec<String>> = client.ft_syndump("index").await?;
    assert_eq!(2, result.len());
    let hello_result = result.get("hello").unwrap();
    assert_eq!(1, hello_result.len());
    assert_eq!("group1", hello_result[0]);
    let world_result = result.get("world").unwrap();
    assert_eq!(1, world_result.len());
    assert_eq!("group1", world_result[0]);

    // search => foo and bar are matched!
    let result = client
        .ft_search("index", "hello", FtSearchOptions::default())
        .await?;
    assert_eq!(2, result.total_results);
    assert_eq!("foo", result.results[0].document_id);
    assert_eq!(
        ("t".to_owned(), "hello".to_owned()),
        result.results[0].values[0]
    );
    assert_eq!("bar", result.results[1].document_id);
    assert_eq!(
        ("t".to_owned(), "world".to_owned()),
        result.results[1].values[0]
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_tagvals() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // Insert documents
    client.hset("foo", ("tag", "hello")).await?;
    client.hset("bar", ("tag", "world")).await?;

    // Create an index
    client
        .ft_create(
            "index",
            FtCreateOptions::default(),
            FtFieldSchema::identifier("tag").field_type(FtFieldType::Tag),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    // Get Tags
    let tags: HashSet<String> = client.ft_tagvals("index", "tag").await?;
    assert!(tags.contains("hello"));
    assert!(tags.contains("world"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_sugadd() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_sugadd(
            "key",
            "hello world",
            1.,
            FtSugAddOptions::default().incr().payload("foo"),
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_sugdel() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_sugadd("key", "hello world", 1., FtSugAddOptions::default())
        .await?;

    let deleted = client.ft_sugdel("key", "hello world").await?;
    assert!(deleted);

    let deleted = client.ft_sugdel("key", "hello world").await?;
    assert!(!deleted);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_sugget() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_sugadd(
            "key",
            "hello",
            1.,
            FtSugAddOptions::default().payload("world"),
        )
        .await?;
    client
        .ft_sugadd("key", "hell", 1., FtSugAddOptions::default().payload("42"))
        .await?;

    let suggestions = client
        .ft_sugget("key", "hell", FtSugGetOptions::default().withpayload())
        .await?;
    assert_eq!("hell".to_owned(), suggestions[0].suggestion);
    assert_eq!("42".to_owned(), suggestions[0].payload);
    assert_eq!(0.0, suggestions[0].score);
    assert_eq!("hello".to_owned(), suggestions[1].suggestion);
    assert_eq!("world".to_owned(), suggestions[1].payload);
    assert_eq!(0.0, suggestions[1].score);

    let suggestions = client
        .ft_sugget(
            "key",
            "hell",
            FtSugGetOptions::default().withpayload().withscores(),
        )
        .await?;
    assert_eq!("hell".to_owned(), suggestions[0].suggestion);
    assert_eq!("42".to_owned(), suggestions[0].payload);
    assert!(suggestions[0].score > 0.);
    assert_eq!("hello".to_owned(), suggestions[1].suggestion);
    assert_eq!("world".to_owned(), suggestions[1].payload);
    assert!(suggestions[1].score > 0.);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_suglen() -> Result<()> {
    let client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_sugadd("key", "hello", 1., FtSugAddOptions::default())
        .await?;

    client
        .ft_sugadd("key", "hell", 1., FtSugAddOptions::default())
        .await?;

    let len = client.ft_suglen("key").await?;
    assert_eq!(2, len);

    Ok(())
}
