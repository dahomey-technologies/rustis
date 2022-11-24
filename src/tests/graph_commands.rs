use crate::{
    resp::{BulkString, Value},
    tests::get_redis_stack_test_client,
    Error, FlushingMode, GraphCommands, GraphQueryOptions, GraphValue, Result, ServerCommands, GraphSlowlogResult,
};
use serial_test::serial;
use std::collections::{HashMap, HashSet};

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_config() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let all_configs: HashMap<String, Value> = client.graph_config_get("*").await?;
    log::debug!("all_configs: {all_configs:?}");
    assert!(!all_configs.is_empty());

    let Some(Value::Integer(timeout)) = all_configs.get("TIMEOUT") else {
        return Err(Error::Client("Cannot find config TIMEOUT".to_owned()));
    };

    client.graph_config_set("TIMEOUT", *timeout / 2).await?;

    let configs: HashMap<String, Value> = client.graph_config_get("TIMEOUT").await?;
    assert_eq!(1, configs.len());

    let Some(Value::Integer(new_timeout)) = configs.get("TIMEOUT") else {
        return Err(Error::Client("Cannot find config TIMEOUT".to_owned()));
    };

    assert_eq!(*timeout / 2, *new_timeout);

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_delete() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.graph_delete("graph").await;
    assert!(result.is_err());

    client
        .graph_query("graph", "CREATE ()", GraphQueryOptions::default())
        .await?;

    client.graph_delete("graph").await?;

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_explain() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.graph_delete("graph").await;
    assert!(result.is_err());

    client
        .graph_query(
            "graph",
            "CREATE (:plant {name: 'Tree'})-[:GROWS {season: 'Autumn'}]->(:fruit {name: 'Apple'})",
            GraphQueryOptions::default(),
        )
        .await?;

    let result: Vec<String> = client
        .graph_explain("graph", "MATCH (a)-[e]->(b) RETURN a, e, b.name")
        .await?;
    assert!(!result.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_list() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.graph_delete("graph").await;
    assert!(result.is_err());

    client
        .graph_query("graph1", "CREATE ()", GraphQueryOptions::default())
        .await?;

    client
        .graph_query("graph2", "CREATE ()", GraphQueryOptions::default())
        .await?;

    let result: HashSet<String> = client.graph_list().await?;
    assert!(result.contains("graph1"));
    assert!(result.contains("graph2"));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_profile() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.graph_delete("graph").await;
    assert!(result.is_err());

    client
        .graph_query(
            "graph",
            "CREATE (:plant {name: 'Tree'})-[:GROWS {season: 'Autumn'}]->(:fruit {name: 'Apple'})",
            GraphQueryOptions::default(),
        )
        .await?;

    let result: Vec<String> = client
        .graph_profile(
            "graph",
            "MATCH (a)-[e]->(b) RETURN a, e, b.name",
            GraphQueryOptions::default(),
        )
        .await?;
    assert!(!result.is_empty());

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_query() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client
        .graph_query(
            "graph",
            r#"CREATE (:object { 
                null: null, 
                string: 'string', 
                integer: 12, 
                boolean: true, 
                double: 12.12, 
                point: point({latitude: 43.642801, longitude: 3.930377}),
                array: ['a', 1, 1.5, false]
            })"#,
            GraphQueryOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(0, result.header.column_names.len());
    assert_eq!(0, result.rows.len());

    // map
    let result = client
        .graph_query(
            "graph",
            "WITH {key1: 'stringval', key2: 10} AS map RETURN map",
            GraphQueryOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(1, result.header.column_names.len());
    assert_eq!(1, result.rows.len());
    assert_eq!(1, result.rows[0].values.len());
    assert_eq!("map", result.header.column_names[0]);
    assert_eq!(
        GraphValue::Map(HashMap::<String, GraphValue>::from([
            (
                "key1".to_owned(),
                GraphValue::String(BulkString::Binary("stringval".as_bytes().to_vec()))
            ),
            ("key2".to_owned(), GraphValue::Integer(10))
        ])),
        result.rows[0].values[0]
    );

    // primitive types
    let result = client
        .graph_query(
            "graph",
            "MATCH (a) RETURN a.null, a.string, a.integer, a.boolean, a.double, a.point, a.array",
            GraphQueryOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(7, result.header.column_names.len());
    assert_eq!(1, result.rows.len());
    assert_eq!(7, result.rows[0].values.len());
    assert_eq!("a.null", result.header.column_names[0]);
    assert_eq!(GraphValue::Null, result.rows[0].values[0]);
    assert_eq!("a.string", result.header.column_names[1]);
    assert_eq!(
        GraphValue::String(BulkString::Binary("string".as_bytes().to_vec())),
        result.rows[0].values[1]
    );
    assert_eq!("a.integer", result.header.column_names[2]);
    assert_eq!(GraphValue::Integer(12), result.rows[0].values[2]);
    assert_eq!("a.boolean", result.header.column_names[3]);
    assert_eq!(GraphValue::Boolean(true), result.rows[0].values[3]);
    assert_eq!("a.double", result.header.column_names[4]);
    assert_eq!(GraphValue::Double(12.12), result.rows[0].values[4]);
    assert_eq!("a.point", result.header.column_names[5]);
    assert_eq!(
        GraphValue::Point((43.642_8, 3.930377)),
        result.rows[0].values[5]
    );
    assert_eq!("a.array", result.header.column_names[6]);
    assert_eq!(
        GraphValue::Array(vec![
            GraphValue::String(BulkString::Binary("a".as_bytes().to_vec())),
            GraphValue::Integer(1),
            GraphValue::Double(1.5),
            GraphValue::Boolean(false)
        ]),
        result.rows[0].values[6]
    );

    // node
    let result = client
        .graph_query("graph", "MATCH (a) RETURN a", GraphQueryOptions::default())
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(1, result.header.column_names.len());
    assert_eq!(1, result.rows.len());
    assert_eq!(1, result.rows[0].values.len());
    assert_eq!("a", result.header.column_names[0]);
    assert!(matches!(result.rows[0].values[0], GraphValue::Node(_)));

    let result = client
        .graph_query(
            "graph",
            "CREATE (:plant {name: 'Tree'})-[:GROWS {season: 'Autumn'}]->(:fruit {name: 'Apple'})",
            GraphQueryOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");

    // node, relationship, primitive
    let result = client
        .graph_query(
            "graph",
            "MATCH (a)-[e]->(b) RETURN a, e, b.name",
            GraphQueryOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(
        vec!["a".to_owned(), "e".to_owned(), "b.name".to_owned()],
        result.header.column_names
    );
    assert_eq!(1, result.rows.len());
    assert_eq!(3, result.rows[0].values.len());
    assert!(matches!(result.rows[0].values[0], GraphValue::Node(_)));
    assert!(matches!(result.rows[0].values[1], GraphValue::Edge(_)));
    assert!(matches!(result.rows[0].values[2], GraphValue::String(_)));

    // path
    let result = client
        .graph_query(
            "graph",
            "MATCH p=(:plant)-[*]->(:fruit) RETURN p",
            GraphQueryOptions::default(),
        )
        .await?;
    log::debug!("result: {result:?}");
    assert_eq!(vec!["p".to_owned()], result.header.column_names);
    assert_eq!(1, result.rows.len());
    assert_eq!(1, result.rows[0].values.len());
    assert!(matches!(result.rows[0].values[0], GraphValue::Path(_)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_query_ro() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.graph_delete("graph").await;
    assert!(result.is_err());

    client
        .graph_query(
            "graph",
            "CREATE (:plant {name: 'Tree'})-[:GROWS {season: 'Autumn'}]->(:fruit {name: 'Apple'})",
            GraphQueryOptions::default(),
        )
        .await?;

    let result = client
        .graph_ro_query(
            "graph",
            "MATCH (a)-[e]->(b) RETURN a, e, b.name",
            GraphQueryOptions::default(),
        )
        .await?;
    assert_eq!(
        vec!["a".to_owned(), "e".to_owned(), "b.name".to_owned()],
        result.header.column_names
    );
    assert_eq!(1, result.rows.len());
    assert_eq!(3, result.rows[0].values.len());
    assert!(matches!(result.rows[0].values[0], GraphValue::Node(_)));
    assert!(matches!(result.rows[0].values[1], GraphValue::Edge(_)));
    assert!(matches!(result.rows[0].values[2], GraphValue::String(_)));

    Ok(())
}

#[cfg_attr(feature = "tokio-runtime", tokio::test)]
#[cfg_attr(feature = "async-std-runtime", async_std::test)]
#[serial]
async fn graph_slowlog() -> Result<()> {
    let mut client = get_redis_stack_test_client().await?;
    client.flushall(FlushingMode::Sync).await?;

    let result = client.graph_delete("graph").await;
    assert!(result.is_err());

    client
        .graph_query(
            "graph",
            "CREATE (:plant {name: 'Tree'})-[:GROWS {season: 'Autumn'}]->(:fruit {name: 'Apple'})",
            GraphQueryOptions::default(),
        )
        .await?;

    let _result = client
        .graph_query(
            "graph",
            "MATCH (a)-[e]->(b) RETURN a, e, b.name",
            GraphQueryOptions::default(),
        )
        .await?;

    let slowlogs: Vec<GraphSlowlogResult> = client.graph_slowlog("graph").await?;
    log::debug!("slowlogs: {slowlogs:?}");
    assert!(!slowlogs.is_empty());

    Ok(())
}
