use crate::{
    Result,
    client::{BatchPreparedCommand, Client},
    commands::{
        ClientReplyMode, ConnectionCommands, FlushingMode, FtAggregateOptions, FtAttribute,
        FtCreateOptions, FtFieldSchema, FtFieldType, FtFlatVectorFieldAttributes,
        FtGeoShapeCoordSystem, FtGroupBy, FtHybridCombine, FtHybridFormat, FtHybridOptions,
        FtHybridSearch, FtHybridVectorQuery, FtHybridVsim, FtIndexAll, FtIndexDataType, FtLanguage,
        FtPhoneticMatcher, FtReducer, FtSearchOptions, FtSearchResult, FtSortBy, FtSortByProperty,
        FtSpellCheckOptions, FtSugAddOptions, FtSugGetOptions, FtTermType, FtVectorDistanceMetric,
        FtVectorFieldAlgorithm, FtVectorType, FtWithCursorOptions, HashCommands, JsonCommands,
        SearchCommands, ServerCommands, SortOrder,
    },
    network::sleep,
    resp::{RefBulkString, Value},
    tests::{TestClient, get_test_client, log_try_init},
};
use rand::{Rng, seq::IndexedRandom};
use serial_test::serial;
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

/// `FtAggregateOptions` nests inline `SmallVec` buffers two levels deep, and its
/// builder methods take `self` by value, so every chained call copies the whole
/// struct on the stack. Keeping the type small is what stops a builder chain from
/// costing hundreds of kilobytes of stack in a debug build.
#[test]
fn ft_aggregate_options_stay_small() {
    assert!(
        size_of::<FtAggregateOptions<'_>>() <= 4096,
        "FtAggregateOptions grew to {} bytes",
        size_of::<FtAggregateOptions<'_>>()
    );
    assert!(
        size_of::<FtGroupBy<'_>>() <= 1024,
        "FtGroupBy grew to {} bytes",
        size_of::<FtGroupBy<'_>>()
    );
}

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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .json_set(
            "doc:1",
            "$",
            r#"[{"arr": [1, 2, 3]}, {"val": "hello"}, {"val": "world"}]"#,
            None,
        )
        .await?;

    client
        .ft_create(
            "idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Json)
                .prefix("doc")
                .schema(
                    FtFieldSchema::identifier("$..arr")
                        .as_attribute("arr")
                        .field_type(FtFieldType::Numeric),
                )
                .schema(
                    FtFieldSchema::identifier("$..val")
                        .as_attribute("val")
                        .field_type(FtFieldType::Text),
                ),
        )
        .await?;
    wait_for_index_scanned(&client, "idx").await?;

    let _result = client
        .ft_aggregate(
            "idx",
            "*",
            FtAggregateOptions::default()
                .load(FtAttribute::new("arr"))
                .load(FtAttribute::new("val")),
        )
        .await?;

    let _result = client
        .ft_aggregate(
            "idx1",
            r#"@url:"about.html""#,
            FtAggregateOptions::default()
                .apply("day(@timestamp)", "day")
                .groupby(
                    FtGroupBy::default()
                        .property("@day")
                        .property("@country")
                        .reduce(FtReducer::count().as_name("num_visits")),
                )
                .sortby(FtSortBy::default().property(FtSortByProperty::new("@day"))),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "books-idx",
            "*",
            FtAggregateOptions::default()
                .groupby(
                    FtGroupBy::default()
                        .property("@published_year")
                        .reduce(FtReducer::count().as_name("num_published")),
                )
                .groupby(FtGroupBy::default().reduce(
                    FtReducer::max("@num_published").as_name("max_books_published_per_year"),
                )),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "libraries-idx",
            "@location:[-73.982254 40.753181 10 km]",
            FtAggregateOptions::default()
                .load(FtAttribute::new("@location"))
                .apply("geodistance(@location, -73.982254, 40.753181)", "day"),
        )
        .await;

    let _result = client
        .ft_aggregate(
            "gh",
            "*",
            FtAggregateOptions::default()
                .groupby(
                    FtGroupBy::default()
                        .property("@actor")
                        .reduce(FtReducer::count().as_name("num")),
                )
                .sortby(
                    FtSortBy::default()
                        .property(FtSortByProperty::new("@day").desc())
                        .max(10),
                ),
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
            FtAggregateOptions::default().groupby(FtGroupBy::default().reduce(
                FtReducer::first_value_by_order("@name", "@age", SortOrder::Desc),
            )),
        )
        .await;

    // example from Redis official documentation
    // https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/aggregations/#example-1-unique-users-by-hour-ordered-chronologically
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
                .prefix("log")
                .schema(
                    FtFieldSchema::identifier("url")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("timestamp")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("country")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("user_id")
                        .field_type(FtFieldType::Text)
                        .noindex()
                        .sortable(),
                ),
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
                    FtGroupBy::default()
                        .property("@hour")
                        .reduce(FtReducer::count_distinct("@user_id").as_name("num_users")),
                )
                .sortby(FtSortBy::default().property(FtSortByProperty::new("@hour").asc()))
                .apply("timefmt(@hour)", "hour"),
        )
        .await?;

    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());
    assert_eq!(2, result.results[0].extra_attributes.len());
    assert_eq!(2, result.results[1].extra_attributes.len());
    assert_eq!(
        ("hour".to_owned(), "2022-11-16T22:00:00Z".to_owned()),
        result.results[0].extra_attributes[0]
    );
    assert_eq!(
        ("num_users".to_owned(), "2".to_owned()),
        result.results[0].extra_attributes[1]
    );
    assert_eq!(
        ("hour".to_owned(), "2022-11-17T03:00:00Z".to_owned()),
        result.results[1].extra_attributes[0]
    );
    assert_eq!(
        ("num_users".to_owned(), "2".to_owned()),
        result.results[1].extra_attributes[1]
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_alias() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("field").field_type(FtFieldType::Text)),
        )
        .await?;
    wait_for_index_scanned(&client, "idx1").await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("field").field_type(FtFieldType::Text)),
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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("field1").field_type(FtFieldType::Text)),
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
    let client = get_test_client().await?;
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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .schema(
                    FtFieldSchema::identifier("title")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("published_at")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("category")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                ),
        )
        .await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("blog:post")
                .schema(
                    FtFieldSchema::identifier("sku")
                        .as_attribute("sku_text")
                        .field_type(FtFieldType::Text),
                )
                .schema(
                    FtFieldSchema::identifier("sku")
                        .as_attribute("sku_tag")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                ),
        )
        .await?;

    client
        .ft_create(
            "author-books-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("author:details:")
                .prefix("book:details:")
                .schema(FtFieldSchema::identifier("author_id").field_type(FtFieldType::Tag))
                .schema(FtFieldSchema::identifier("title").field_type(FtFieldType::Text))
                .schema(FtFieldSchema::identifier("name").field_type(FtFieldType::Text)),
        )
        .await?;

    client
        .ft_create(
            "g-authors-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("author:details")
                .filter(r#"startswith(@name, "G")"#)
                .schema(FtFieldSchema::identifier("name").field_type(FtFieldType::Text)),
        )
        .await?;

    client
        .ft_create(
            "subtitled-books-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("book:details")
                .filter(r#"@subtitle != """#)
                .schema(FtFieldSchema::identifier("title").field_type(FtFieldType::Text)),
        )
        .await?;

    client
        .ft_create(
            "books-idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("book:details")
                .schema(FtFieldSchema::identifier("title").field_type(FtFieldType::Text))
                .schema(
                    FtFieldSchema::identifier("categories")
                        .field_type(FtFieldType::Tag)
                        .separator(';'),
                ),
        )
        .await?;

    client
        .ft_create(
            "idx3",
            FtCreateOptions::default()
                .on(FtIndexDataType::Json)
                .prefix("book:details")
                .schema(
                    FtFieldSchema::identifier("$.title")
                        .as_attribute("title")
                        .field_type(FtFieldType::Text),
                )
                .schema(
                    FtFieldSchema::identifier("$.categories")
                        .as_attribute("categories")
                        .field_type(FtFieldType::Tag),
                ),
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
                .score(1.0)
                .schema(
                    FtFieldSchema::identifier("$.model")
                        .field_type(FtFieldType::Text)
                        .weight(1.0)
                        .nostem(),
                )
                .schema(
                    FtFieldSchema::identifier("$.brand")
                        .field_type(FtFieldType::Text)
                        .weight(1.0)
                        .nostem(),
                )
                .schema(FtFieldSchema::identifier("$.price").field_type(FtFieldType::Numeric))
                .schema(
                    FtFieldSchema::identifier("$.type")
                        .field_type(FtFieldType::Tag)
                        .separator(','),
                )
                .schema(
                    FtFieldSchema::identifier("$.description")
                        .as_attribute("description")
                        .field_type(FtFieldType::Text)
                        .weight(1.0)
                        .nostem(),
                )
                .schema(
                    FtFieldSchema::identifier("$.description_embeddings ").field_type(
                        FtFieldType::Vector(Some(FtVectorFieldAlgorithm::Flat(
                            FtFlatVectorFieldAttributes::new(
                                FtVectorType::Float32,
                                768,
                                FtVectorDistanceMetric::Cosine,
                            ),
                        ))),
                    ),
                ),
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_hybrid() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // A small hash index with a text field and a 4-dimensional FLAT float32 vector.
    client
        .ft_create(
            "hybrid_idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("doc:")
                .schema(FtFieldSchema::identifier("content").field_type(FtFieldType::Text))
                .schema(
                    FtFieldSchema::identifier("embedding").field_type(FtFieldType::Vector(Some(
                        FtVectorFieldAlgorithm::Flat(FtFlatVectorFieldAttributes::new(
                            FtVectorType::Float32,
                            4,
                            FtVectorDistanceMetric::L2,
                        )),
                    ))),
                ),
        )
        .await?;

    let embedding = |v: [f32; 4]| v.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();

    let doc1 = embedding([1.0, 0.0, 0.0, 0.0]);
    let doc2 = embedding([0.0, 1.0, 0.0, 0.0]);
    client
        .hset(
            "doc:1",
            [
                ("content", RefBulkString::new(b"red bicycle")),
                ("embedding", RefBulkString::new(&doc1)),
            ],
        )
        .await?;
    client
        .hset(
            "doc:2",
            [
                ("content", RefBulkString::new(b"blue car")),
                ("embedding", RefBulkString::new(&doc2)),
            ],
        )
        .await?;

    wait_for_index_scanned(&client, "hybrid_idx").await?;

    // Hybrid query: text search for "bicycle" fused with a KNN vector search,
    // the query vector supplied through PARAMS.
    let query_vector = embedding([1.0, 0.0, 0.0, 0.0]);
    let result: Value = client
        .ft_hybrid(
            "hybrid_idx",
            FtHybridSearch::new("bicycle"),
            FtHybridVsim::new("@embedding", "$vec").query(FtHybridVectorQuery::Knn {
                k: 2,
                ef_runtime: None,
            }),
            FtHybridOptions::default()
                .combine(FtHybridCombine::Rrf {
                    constant: None,
                    window: Some(40),
                })
                .limit(0, 10)
                .load(["@content"])
                .param("vec", &query_vector),
        )
        .await?;

    // A successful hybrid query returns a non-null reply describing the matches.
    assert!(!matches!(result, Value::Null));

    // Advanced post-processing: fuse the two result sets, group the fused rows by
    // their `content`, count each group, sort by that count and cap the output —
    // exercising GROUPBY/REDUCE, the (count-aware) SORTBY, LIMIT and FORMAT.
    let grouped: Value = client
        .ft_hybrid(
            "hybrid_idx",
            FtHybridSearch::new("bicycle"),
            FtHybridVsim::new("@embedding", "$vec").query(FtHybridVectorQuery::Knn {
                k: 2,
                ef_runtime: None,
            }),
            FtHybridOptions::default()
                .combine(FtHybridCombine::Rrf {
                    constant: None,
                    window: Some(40),
                })
                .load(["@content"])
                .groupby(
                    FtGroupBy::default()
                        .property("@content")
                        .reduce(FtReducer::count().as_name("cnt")),
                )
                .sortby("@cnt", SortOrder::Desc)
                .limit(0, 10)
                .format(FtHybridFormat::String)
                .param("vec", &query_vector),
        )
        .await?;
    assert!(!matches!(grouped, Value::Null));

    // Post-combine FILTER on an APPLY-computed field.
    let filtered: Value = client
        .ft_hybrid(
            "hybrid_idx",
            FtHybridSearch::new("bicycle"),
            FtHybridVsim::new("@embedding", "$vec").query(FtHybridVectorQuery::Knn {
                k: 2,
                ef_runtime: None,
            }),
            FtHybridOptions::default()
                .combine(FtHybridCombine::Rrf {
                    constant: None,
                    window: Some(40),
                })
                .load(["@content"])
                .apply("upper(@content)", "upper_content")
                .filter("@upper_content != ''")
                .limit(0, 10)
                .param("vec", &query_vector),
        )
        .await?;
    assert!(!matches!(filtered, Value::Null));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_cursor() -> Result<()> {
    let client = get_test_client().await?;
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
                        format!("page{}.html", rand::rng().random_range(1..21)).to_owned(),
                    ),
                    ("timestamp", (1668637156 + i).to_string()),
                    (
                        "country",
                        (*["fr", "ca"].choose(&mut rand::rng()).unwrap()).to_owned(),
                    ),
                    (
                        "user_id",
                        format!("user{}", rand::rng().random_range(1..11)),
                    ),
                ],
            )
            .forget();
    }

    pipeline.client_reply(ClientReplyMode::On).forget();

    pipeline.execute::<()>().await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("log")
                .schema(
                    FtFieldSchema::identifier("url")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("timestamp")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("country")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("user_id")
                        .field_type(FtFieldType::Text)
                        .noindex()
                        .sortable(),
                ),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_aggregate(
            "index",
            "*",
            FtAggregateOptions::default()
                .groupby(
                    FtGroupBy::default()
                        .property("@url")
                        .reduce(FtReducer::count_distinct("@user_id").as_name("num_users")),
                )
                .sortby(FtSortBy::default().property(FtSortByProperty::new("@num_users").desc()))
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
    let client = get_test_client().await?;
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
    let client = get_test_client().await?;
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
                .prefix("log")
                .schema(
                    FtFieldSchema::identifier("url")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("timestamp")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("country")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("user_id")
                        .field_type(FtFieldType::Text)
                        .noindex()
                        .sortable(),
                ),
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
                .prefix("log")
                .schema(
                    FtFieldSchema::identifier("url")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("timestamp")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("country")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("user_id")
                        .field_type(FtFieldType::Text)
                        .noindex()
                        .sortable(),
                ),
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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .schema(
                    FtFieldSchema::identifier("text")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("date")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                ),
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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .schema(
                    FtFieldSchema::identifier("text")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("date")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                ),
        )
        .await?;

    let execution_plan = client
        .ft_explaincli(
            "index",
            "(foo bar)|(hello world) @date:[100 200]|@date:[500 +inf]",
            None,
        )
        .await?;
    assert!(matches!(execution_plan, Value::Array(array) if !array.is_empty()));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_info() -> Result<()> {
    log_try_init();

    let client = get_test_client().await?;
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
                .nofreqs()
                .prefix("log")
                .prefix("doc")
                .skip_initial_scan()
                .stop_word("hello")
                .stop_word("world")
                .schema(
                    FtFieldSchema::identifier("text")
                        .field_type(FtFieldType::Text)
                        .phonetic(FtPhoneticMatcher::DmEn)
                        .nostem()
                        .sortable()
                        .unf(),
                )
                .schema(
                    FtFieldSchema::identifier("date")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                ),
        )
        .await?;

    let info = client.ft_info("index").await?;
    tracing::debug!("info: {info:?}");

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_list() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx1",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("field").field_type(FtFieldType::Text)),
        )
        .await?;

    client
        .ft_create(
            "idx2",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("field").field_type(FtFieldType::Text)),
        )
        .await?;

    client
        .ft_create(
            "idx3",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("field").field_type(FtFieldType::Text)),
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
    let client = get_test_client().await?;
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
                        format!("page{}.html", rand::rng().random_range(1..21)).to_owned(),
                    ),
                    ("timestamp", (1668637156 + i).to_string()),
                    (
                        "country",
                        (*["fr", "ca"].choose(&mut rand::rng()).unwrap()).to_owned(),
                    ),
                    (
                        "user_id",
                        format!("user{}", rand::rng().random_range(1..11)),
                    ),
                ],
            )
            .forget();
    }

    pipeline.client_reply(ClientReplyMode::On).forget();

    pipeline.execute::<()>().await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("log")
                .schema(
                    FtFieldSchema::identifier("url")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("timestamp")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("country")
                        .field_type(FtFieldType::Tag)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("user_id")
                        .field_type(FtFieldType::Text)
                        .noindex()
                        .sortable(),
                ),
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

    tracing::debug!("result: {result:?}");

    // Under RESP3, FT.PROFILE answers a map holding the query's own reply next
    // to the profiling report.
    let result = client.ft_profile_search("index", false, "*").await?;
    tracing::debug!("result: {result:?}");
    let Value::Map(parts) = result else {
        panic!("expected a map, got {result:?}");
    };
    assert!(parts.contains_key(&Value::SimpleString("Profile".to_owned())));
    assert!(parts.contains_key(&Value::SimpleString("Results".to_owned())));

    let result = client.ft_profile_search("index", true, "*").await?;
    assert!(matches!(result, Value::Map(_)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_search() -> Result<()> {
    let client = get_test_client().await?;
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
                .payload_field("payload")
                .schema(
                    FtFieldSchema::identifier("title")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("data")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("published_at")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                ),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_search("index", "wizard", FtSearchOptions::default())
        .await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    let result = client
        .ft_search("index", "@title:dogs", FtSearchOptions::default())
        .await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(1, result.total_results);
    assert_eq!(1, result.results.len());

    let result = client
        .ft_search(
            "index",
            "@published_at:[2020 2021]",
            FtSearchOptions::default(),
        )
        .await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(1, result.total_results);
    assert_eq!(1, result.results.len());

    let result = client
        .ft_search("index", "*", FtSearchOptions::default().nocontent())
        .await?;
    tracing::debug!("result: {result:?}");
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
                .sortby("title", SortOrder::Asc, false),
        )
        .await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    // with pipeline
    let mut pipeline = client.create_pipeline();
    pipeline
        .ft_search("index", "wizard", FtSearchOptions::default())
        .queue();
    let result: FtSearchResult = pipeline.execute().await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(2, result.total_results);
    assert_eq!(2, result.results.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_search_empty_index() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("doc")
                .payload_field("payload")
                .schema(
                    FtFieldSchema::identifier("title")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("data")
                        .field_type(FtFieldType::Text)
                        .sortable(),
                )
                .schema(
                    FtFieldSchema::identifier("published_at")
                        .field_type(FtFieldType::Numeric)
                        .sortable(),
                ),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_search("index", "wizard", FtSearchOptions::default())
        .await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(0, result.total_results);
    assert_eq!(0, result.results.len());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_spellcheck() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client.hset("doc", ("text", "hello help")).await?;
    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("text").field_type(FtFieldType::Text)),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    let result = client
        .ft_spellcheck("index", "held", FtSpellCheckOptions::default().distance(2))
        .await?;

    assert_eq!(1, result.misspelled_terms.len());
    assert_eq!("held", result.misspelled_terms[0].misspelled_term);
    assert_eq!(2, result.misspelled_terms[0].suggestions.len());
    assert!(
        result.misspelled_terms[0]
            .suggestions
            .iter()
            .any(|(suggestion, _score)| suggestion == "hello")
    );
    assert!(
        result.misspelled_terms[0]
            .suggestions
            .iter()
            .any(|(suggestion, _score)| suggestion == "help")
    );

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
    assert_eq!("help", result.misspelled_terms[0].suggestions[0].0);
    assert_eq!("stor", result.misspelled_terms[1].misspelled_term);
    assert_eq!(1, result.misspelled_terms[1].suggestions.len());
    assert_eq!("store", result.misspelled_terms[1].suggestions[0].0);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_syn() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // Insert documents
    client.hset("foo", ("t", "hello")).await?;
    client.hset("bar", ("t", "world")).await?;

    // Create an index
    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("t").field_type(FtFieldType::Text)),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    // search => only foo is matched
    let result = client
        .ft_search("index", "hello", FtSearchOptions::default())
        .await?;
    tracing::debug!("result: {result:?}");
    assert_eq!(1, result.total_results);
    assert_eq!(1, result.results.len());
    assert_eq!("foo", result.results[0].id);
    assert_eq!(1, result.results[0].extra_attributes.len());
    assert_eq!(
        ("t".to_owned(), "hello".to_owned()),
        result.results[0].extra_attributes[0]
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

    // FT.SYNUPDATE triggers an asynchronous re-scan of already-indexed
    // documents to apply the new synonym group, so wait for it to complete
    // before searching (otherwise the search may still return the old result).
    wait_for_index_scanned(&client, "index").await?;

    // search => foo and bar are matched!
    let mut result = client
        .ft_search("index", "hello", FtSearchOptions::default())
        .await?;
    for _ in 0..50 {
        if result.total_results == 2 {
            break;
        }
        sleep(Duration::from_millis(100)).await;
        result = client
            .ft_search("index", "hello", FtSearchOptions::default())
            .await?;
    }
    assert_eq!(2, result.total_results);
    assert_eq!("foo", result.results[0].id);
    assert_eq!(1, result.results[0].extra_attributes.len());
    assert_eq!(
        ("t".to_owned(), "hello".to_owned()),
        result.results[0].extra_attributes[0]
    );
    assert_eq!("bar", result.results[1].id);
    assert_eq!(1, result.results[1].extra_attributes.len());
    assert_eq!(
        ("t".to_owned(), "world".to_owned()),
        result.results[1].extra_attributes[0]
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_tagvals() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    // Insert documents
    client.hset("foo", ("tag", "hello")).await?;
    client.hset("bar", ("tag", "world")).await?;

    // Create an index
    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("tag").field_type(FtFieldType::Tag)),
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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_sugadd(
            "key",
            "hello world",
            1.,
            FtSugAddOptions::default().incr().payload(b"foo"),
        )
        .await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_sugdel() -> Result<()> {
    let client = get_test_client().await?;
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
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_sugadd(
            "key",
            "hello",
            1.,
            FtSugAddOptions::default().payload(b"world"),
        )
        .await?;
    client
        .ft_sugadd("key", "hell", 1., FtSugAddOptions::default().payload(b"42"))
        .await?;

    let suggestions: Vec<Value> = client
        .ft_sugget("key", "hell", FtSugGetOptions::default().withpayloads())
        .await?;
    assert_eq!(Value::BulkString(b"hell".to_vec()), suggestions[0]);
    assert_eq!(Value::BulkString(b"42".to_vec()), suggestions[1]);
    assert_eq!(Value::BulkString(b"hello".to_vec()), suggestions[2]);
    assert_eq!(Value::BulkString(b"world".to_vec()), suggestions[3]);

    let suggestions: Vec<Value> = client
        .ft_sugget(
            "key",
            "hell",
            FtSugGetOptions::default().withpayloads().withscores(),
        )
        .await?;
    assert_eq!(Value::BulkString(b"hell".to_vec()), suggestions[0]);
    assert!(matches!(suggestions[1], Value::Double(d) if d > 0.));
    assert_eq!(Value::BulkString(b"42".to_vec()), suggestions[2]);
    assert_eq!(Value::BulkString(b"hello".to_vec()), suggestions[3]);
    assert!(matches!(suggestions[4], Value::Double(d) if d > 0.));
    assert_eq!(Value::BulkString(b"world".to_vec()), suggestions[5]);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_suglen() -> Result<()> {
    let client = get_test_client().await?;
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

/// `FT.AGGREGATE ... LOAD count ...` — `count` is the number of arguments that
/// follow, so an attribute renamed with `AS` accounts for three of them.
#[test]
fn ft_aggregate_load_args() -> Result<()> {
    let cmd = TestClient
        .ft_aggregate(
            "index",
            "*",
            FtAggregateOptions::default()
                .load(FtAttribute::new("@a"))
                .load(FtAttribute::new("@b").r#as("c")),
        )
        .command;
    assert_eq!("FT.AGGREGATE index * LOAD 4 @a @b AS c", &cmd.to_string());

    Ok(())
}

/// `FT.SEARCH ... RETURN count ...` — same argument-counting rule as LOAD.
#[test]
fn ft_search_return_args() -> Result<()> {
    let cmd = TestClient
        .ft_search(
            "index",
            "*",
            FtSearchOptions::default()
                ._return(FtAttribute::new("@a"))
                ._return(FtAttribute::new("@b").r#as("c")),
        )
        .command;
    assert_eq!("FT.SEARCH index * RETURN 4 @a @b AS c", &cmd.to_string());

    Ok(())
}

/// `PARAMS nargs name value ...` — `nargs` counts every token, so two per pair.
#[test]
fn ft_aggregate_params_args() -> Result<()> {
    let cmd = TestClient
        .ft_aggregate(
            "index",
            "*",
            FtAggregateOptions::default()
                .param("n1", "v1")
                .param("n2", "v2"),
        )
        .command;
    assert_eq!(
        "FT.AGGREGATE index * PARAMS 4 n1 v1 n2 v2",
        &cmd.to_string()
    );

    Ok(())
}

#[test]
fn ft_search_params_args() -> Result<()> {
    let cmd = TestClient
        .ft_search(
            "index",
            "*",
            FtSearchOptions::default()
                .param("n1", "v1")
                .param("n2", "v2"),
        )
        .command;
    assert_eq!("FT.SEARCH index * PARAMS 4 n1 v1 n2 v2", &cmd.to_string());

    Ok(())
}

/// `FT.SPELLCHECK ... TERMS {INCLUDE|EXCLUDE} dictionary` takes no count.
#[test]
fn ft_spellcheck_args() -> Result<()> {
    let cmd = TestClient
        .ft_spellcheck(
            "index",
            "query",
            FtSpellCheckOptions::default()
                .distance(2)
                .terms(FtTermType::Include, "dict"),
        )
        .command;
    assert_eq!(
        "FT.SPELLCHECK index query DISTANCE 2 TERMS INCLUDE dict",
        &cmd.to_string()
    );

    Ok(())
}

/// Renaming an attribute with `AS`, and passing more than one query parameter,
/// are the two shapes whose argument count used to be under-reported. Both are
/// rejected outright by the server when the count is wrong, so this exercises
/// them end to end.
#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_renamed_attributes_and_params() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "index",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("doc:")
                .schema(FtFieldSchema::identifier("a").field_type(FtFieldType::Text))
                .schema(FtFieldSchema::identifier("b").field_type(FtFieldType::Text)),
        )
        .await?;
    wait_for_index_scanned(&client, "index").await?;

    client
        .hset("doc:1", [("a", "hello"), ("b", "world")])
        .await?;

    let result = client
        .ft_search(
            "index",
            "@a:($p1) | @b:($p2)",
            FtSearchOptions::default()
                ._return(FtAttribute::new("b").r#as("renamed"))
                .param("p1", "hello")
                .param("p2", "nothing")
                .dialect(2),
        )
        .await?;
    assert_eq!(1, result.total_results);
    assert_eq!(
        vec![("renamed".to_owned(), "world".to_owned())],
        result.results[0].extra_attributes
    );

    let result = client
        .ft_aggregate(
            "index",
            "@a:($p1) | @b:($p2)",
            FtAggregateOptions::default()
                .load(FtAttribute::new("b").r#as("renamed"))
                .param("p1", "hello")
                .param("p2", "nothing")
                .dialect(2),
        )
        .await?;
    assert_eq!(1, result.results.len());
    assert_eq!(
        vec![("renamed".to_owned(), "world".to_owned())],
        result.results[0].extra_attributes
    );

    Ok(())
}

/// `FT.HYBRID` declares an argument count for most of its clauses. This pins the
/// exact bytes of the ones that do, so that deriving those counts cannot change
/// what goes on the wire.
#[test]
fn ft_hybrid_args() -> Result<()> {
    let cmd = TestClient
        .ft_hybrid::<Value>(
            "index",
            FtHybridSearch::new("bicycle").scorer(["BM25"]),
            FtHybridVsim::new("@embedding", "$vec").query(FtHybridVectorQuery::Knn {
                k: 2,
                ef_runtime: Some(30),
            }),
            FtHybridOptions::default()
                .combine(FtHybridCombine::Rrf {
                    constant: Some(60.0),
                    window: Some(40),
                })
                .load(["@content"])
                .sortby("@content", SortOrder::Desc)
                .param("vec", b"ab")
                .param("other", b"cd")
                .param("third", b"ef"),
        )
        .command;
    assert_eq!(
        "FT.HYBRID index SEARCH bicycle SCORER 1 BM25 VSIM @embedding $vec KNN 4 K 2 EF_RUNTIME 30 \
         COMBINE RRF 4 CONSTANT 60.0 WINDOW 40 LOAD 1 @content SORTBY 2 @content DESC \
         PARAMS 6 vec ab other cd third ef",
        &cmd.to_string()
    );

    let cmd = TestClient
        .ft_hybrid::<Value>(
            "index",
            FtHybridSearch::new("bicycle"),
            FtHybridVsim::new("@embedding", "$vec").query(FtHybridVectorQuery::Range {
                radius: 0.5,
                epsilon: None,
            }),
            FtHybridOptions::default().combine(FtHybridCombine::Linear {
                alpha: 0.3,
                beta: 0.7,
                window: None,
            }),
        )
        .command;
    assert_eq!(
        "FT.HYBRID index SEARCH bicycle VSIM @embedding $vec RANGE 2 RADIUS 0.5 \
         COMBINE LINEAR 4 ALPHA 0.3 BETA 0.7",
        &cmd.to_string()
    );

    // `COMBINE RRF` with no argument at all is dropped: RRF is already the default.
    let cmd = TestClient
        .ft_hybrid::<Value>(
            "index",
            FtHybridSearch::new("bicycle"),
            FtHybridVsim::new("@embedding", "$vec"),
            FtHybridOptions::default().combine(FtHybridCombine::Rrf {
                constant: None,
                window: None,
            }),
        )
        .command;
    assert_eq!(
        "FT.HYBRID index SEARCH bicycle VSIM @embedding $vec",
        &cmd.to_string()
    );

    Ok(())
}

/// The `FLAT` and `HNSW` vector field clauses declare the number of attribute
/// tokens that follow them.
#[test]
fn ft_create_vector_field_args() -> Result<()> {
    let cmd = TestClient
        .ft_create(
            "index",
            FtCreateOptions::default().schema(FtFieldSchema::identifier("v").field_type(
                FtFieldType::Vector(Some(FtVectorFieldAlgorithm::Flat(
                    FtFlatVectorFieldAttributes::new(
                        FtVectorType::Float32,
                        4,
                        FtVectorDistanceMetric::L2,
                    ),
                ))),
            )),
        )
        .command;
    assert_eq!(
        "FT.CREATE index SCHEMA v VECTOR FLAT 6 TYPE FLOAT32 DIM 4 DISTANCE_METRIC L2",
        &cmd.to_string()
    );

    let cmd = TestClient
        .ft_create(
            "index",
            FtCreateOptions::default().schema(
                FtFieldSchema::identifier("v").field_type(FtFieldType::Vector(Some(
                    FtVectorFieldAlgorithm::Flat(
                        FtFlatVectorFieldAttributes::new(
                            FtVectorType::Float32,
                            4,
                            FtVectorDistanceMetric::L2,
                        )
                        .initial_cap(100)
                        .block_size(10),
                    ),
                ))),
            ),
        )
        .command;
    assert_eq!(
        "FT.CREATE index SCHEMA v VECTOR FLAT 10 TYPE FLOAT32 DIM 4 DISTANCE_METRIC L2 \
         INITIAL_CAP 100 BLOCK_SIZE 10",
        &cmd.to_string()
    );

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_create_geoshape() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("shape:")
                .schema(
                    FtFieldSchema::identifier("geom")
                        .field_type(FtFieldType::Geoshape(Some(FtGeoShapeCoordSystem::Flat))),
                ),
        )
        .await?;

    client
        .hset(
            "shape:1",
            [("geom", "POLYGON((0 0, 0 10, 10 10, 10 0, 0 0))")],
        )
        .await?;
    client
        .hset(
            "shape:2",
            [("geom", "POLYGON((20 20, 20 30, 30 30, 30 20, 20 20))")],
        )
        .await?;

    sleep(Duration::from_millis(100)).await;

    let result: FtSearchResult = client
        .ft_search(
            "idx",
            "@geom:[WITHIN $shape]",
            FtSearchOptions::default()
                .param("shape", "POLYGON((-1 -1, -1 11, 11 11, 11 -1, -1 -1))")
                .dialect(2),
        )
        .await?;
    assert_eq!(1, result.total_results);
    assert_eq!("shape:1", result.results[0].id);

    Ok(())
}

#[test]
fn ft_create_geoshape_args() {
    // The coordinate system is an optional value following GEOSHAPE, not a flag.
    let cmd = TestClient
        .ft_create(
            "idx",
            FtCreateOptions::default().schema(FtFieldSchema::identifier("geom").field_type(
                FtFieldType::Geoshape(Some(FtGeoShapeCoordSystem::Spherical)),
            )),
        )
        .command;
    assert_eq!(
        "FT.CREATE idx SCHEMA geom GEOSHAPE SPHERICAL",
        cmd.to_string()
    );

    // Omitted, the server defaults to SPHERICAL.
    let cmd = TestClient
        .ft_create(
            "idx",
            FtCreateOptions::default()
                .schema(FtFieldSchema::identifier("geom").field_type(FtFieldType::Geoshape(None))),
        )
        .command;
    assert_eq!("FT.CREATE idx SCHEMA geom GEOSHAPE", cmd.to_string());
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn ft_create_index_missing_and_empty() -> Result<()> {
    let client = get_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    client
        .ft_create(
            "idx",
            FtCreateOptions::default()
                .on(FtIndexDataType::Hash)
                .prefix("doc:")
                .schema(
                    FtFieldSchema::identifier("title")
                        .field_type(FtFieldType::Text)
                        .index_missing()
                        .index_empty(),
                )
                .schema(
                    FtFieldSchema::identifier("category")
                        .field_type(FtFieldType::Tag)
                        .index_missing()
                        .index_empty(),
                ),
        )
        .await?;

    client
        .hset("doc:1", [("title", "hello"), ("category", "a")])
        .await?;
    // An empty value, and a document missing `category` altogether.
    client
        .hset("doc:2", [("title", ""), ("category", "")])
        .await?;
    client.hset("doc:3", [("title", "world")]).await?;

    sleep(Duration::from_millis(100)).await;

    // Without INDEXMISSING, `doc:3` would be unreachable by any query.
    // `ismissing` and the empty-tag syntax both require query dialect 2.
    let result = client
        .ft_search(
            "idx",
            "ismissing(@category)",
            FtSearchOptions::default().dialect(2),
        )
        .await?;
    assert_eq!(1, result.total_results);
    assert_eq!("doc:3", result.results[0].id);

    // Without INDEXEMPTY, the empty tag of `doc:2` would not be indexed.
    let result = client
        .ft_search(
            "idx",
            "@category:{\"\"}",
            FtSearchOptions::default().dialect(2),
        )
        .await?;
    assert_eq!(1, result.total_results);
    assert_eq!("doc:2", result.results[0].id);

    client.ft_dropindex("idx", false).await?;

    Ok(())
}

#[test]
fn ft_create_index_all_args() {
    // INDEXALL carries an explicit value; emitting it as a bare flag is a
    // server-side error.
    let cmd = TestClient
        .ft_create(
            "idx",
            FtCreateOptions::default()
                .index_all(FtIndexAll::Enable)
                .on(FtIndexDataType::Hash)
                .schema(FtFieldSchema::identifier("t").field_type(FtFieldType::Text)),
        )
        .command;
    assert_eq!(
        "FT.CREATE idx ON HASH INDEXALL ENABLE SCHEMA t TEXT",
        cmd.to_string()
    );
}
