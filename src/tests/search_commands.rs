use crate::{
    Result,
    commands::{
        FtAggregateOptions, FtAttribute, FtCreateOptions, FtFieldSchema, FtFieldType,
        FtFlatVectorFieldAttributes, FtGeoShapeCoordSystem, FtGroupBy, FtHnswVectorFieldAttributes,
        FtHybridCombine, FtHybridOptions, FtHybridSearch, FtHybridVectorQuery, FtHybridVsim,
        FtIndexAll, FtIndexDataType, FtReducer, FtSearchOptions, FtSortBy, FtSortByProperty,
        FtSpellCheckOptions, FtTermType, FtVectorDistanceMetric, FtVectorFieldAlgorithm,
        FtVectorType, SearchCommands, SortOrder,
    },
    resp::Value,
    tests::TestClient,
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

/// Every `REDUCE` function carries the number of arguments that follow it, and
/// that count is hard-coded per reducer rather than derived. This pins the bytes
/// of each one.
#[test]
fn ft_reducer_args() -> Result<()> {
    fn reduce_args(reducer: FtReducer<'_>) -> String {
        TestClient
            .ft_aggregate(
                "index",
                "*",
                FtAggregateOptions::default()
                    .groupby(FtGroupBy::default().property("@a").reduce(reducer)),
            )
            .command
            .to_string()
            .replace("FT.AGGREGATE index * GROUPBY 1 @a ", "")
    }

    assert_eq!("REDUCE COUNT 0", reduce_args(FtReducer::count()));
    assert_eq!(
        "REDUCE COUNT_DISTINCT 1 @b",
        reduce_args(FtReducer::count_distinct("@b"))
    );
    assert_eq!(
        "REDUCE COUNT_DISTINCTISH 1 @b",
        reduce_args(FtReducer::count_distinctish("@b"))
    );
    assert_eq!("REDUCE SUM 1 @b", reduce_args(FtReducer::sum("@b")));
    assert_eq!("REDUCE MIN 1 @b", reduce_args(FtReducer::min("@b")));
    assert_eq!("REDUCE MAX 1 @b", reduce_args(FtReducer::max("@b")));
    assert_eq!("REDUCE AVG 1 @b", reduce_args(FtReducer::avg("@b")));
    assert_eq!("REDUCE STDDEV 1 @b", reduce_args(FtReducer::stddev("@b")));
    assert_eq!(
        "REDUCE QUANTILE 2 @b 0.5",
        reduce_args(FtReducer::quantile("@b", 0.5))
    );
    assert_eq!("REDUCE TOLIST 1 @b", reduce_args(FtReducer::tolist("@b")));
    assert_eq!(
        "REDUCE FIRST_VALUE 1 @b",
        reduce_args(FtReducer::first_value("@b"))
    );
    assert_eq!(
        "REDUCE FIRST_VALUE 3 @b BY @c",
        reduce_args(FtReducer::first_value_by("@b", "@c"))
    );
    assert_eq!(
        "REDUCE FIRST_VALUE 4 @b BY @c DESC",
        reduce_args(FtReducer::first_value_by_order("@b", "@c", SortOrder::Desc))
    );
    assert_eq!(
        "REDUCE RANDOM_SAMPLE 2 @b 3",
        reduce_args(FtReducer::random_sample("@b", 3))
    );
    assert_eq!(
        "REDUCE SUM 1 @b AS total",
        reduce_args(FtReducer::sum("@b").as_name("total"))
    );

    Ok(())
}

/// `SORTBY` counts its property tokens, and `WITHCOUNT` / `WITHOUTCOUNT` are
/// flags that follow that count without being part of it.
#[test]
fn ft_sortby_args() -> Result<()> {
    let sortby_args = |sortby: FtSortBy<'_>| {
        TestClient
            .ft_aggregate("index", "*", FtAggregateOptions::default().sortby(sortby))
            .command
            .to_string()
            .replace("FT.AGGREGATE index * ", "")
    };

    assert_eq!(
        "SORTBY 2 @a ASC",
        sortby_args(FtSortBy::default().property(FtSortByProperty::new("@a").asc()))
    );
    assert_eq!(
        "SORTBY 2 @a DESC MAX 10",
        sortby_args(
            FtSortBy::default()
                .property(FtSortByProperty::new("@a").desc())
                .max(10)
        )
    );
    assert_eq!(
        "SORTBY 2 @a ASC WITHCOUNT",
        sortby_args(
            FtSortBy::default()
                .property(FtSortByProperty::new("@a").asc())
                .with_count()
        )
    );
    assert_eq!(
        "SORTBY 2 @a ASC WITHOUTCOUNT",
        sortby_args(
            FtSortBy::default()
                .property(FtSortByProperty::new("@a").asc())
                .without_count()
        )
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
                shard_k_ratio: None,
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

    // `YIELD_SCORE_AS` sits inside each clause, after that clause's own arguments,
    // and is not part of any declared count.
    let cmd = TestClient
        .ft_hybrid::<Value>(
            "index",
            FtHybridSearch::new("bicycle").yield_score_as("text_score"),
            FtHybridVsim::new("@embedding", "$vec")
                .query(FtHybridVectorQuery::Knn {
                    k: 2,
                    ef_runtime: None,
                    shard_k_ratio: None,
                })
                .yield_score_as("vector_score"),
            FtHybridOptions::default(),
        )
        .command;
    assert_eq!(
        "FT.HYBRID index SEARCH bicycle YIELD_SCORE_AS text_score \
         VSIM @embedding $vec KNN 2 K 2 YIELD_SCORE_AS vector_score",
        &cmd.to_string()
    );

    // `SHARD_K_RATIO` is a `KNN` argument and counts towards the clause count.
    let cmd = TestClient
        .ft_hybrid::<Value>(
            "index",
            FtHybridSearch::new("bicycle"),
            FtHybridVsim::new("@embedding", "$vec").query(FtHybridVectorQuery::Knn {
                k: 2,
                ef_runtime: Some(30),
                shard_k_ratio: Some(0.5),
            }),
            FtHybridOptions::default(),
        )
        .command;
    assert_eq!(
        "FT.HYBRID index SEARCH bicycle VSIM @embedding $vec \
         KNN 6 K 2 EF_RUNTIME 30 SHARD_K_RATIO 0.5",
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

    let cmd = TestClient
        .ft_create(
            "index",
            FtCreateOptions::default().schema(
                FtFieldSchema::identifier("v").field_type(FtFieldType::Vector(Some(
                    FtVectorFieldAlgorithm::HNSW(
                        FtHnswVectorFieldAttributes::new(
                            FtVectorType::Float32,
                            4,
                            FtVectorDistanceMetric::Cosine,
                        )
                        .initial_cap(100)
                        .m(16)
                        .ef_construction(200)
                        .ef_runtime(10)
                        .epsilon(0.01),
                    ),
                ))),
            ),
        )
        .command;
    assert_eq!(
        "FT.CREATE index SCHEMA v VECTOR HNSW 16 TYPE FLOAT32 DIM 4 DISTANCE_METRIC COSINE \
         INITIAL_CAP 100 M 16 EF_CONSTRUCTION 200 EF_RUNTIME 10 EPSILON 0.01",
        &cmd.to_string()
    );

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
