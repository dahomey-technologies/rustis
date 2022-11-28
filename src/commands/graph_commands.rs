use crate::{
    prepare_command,
    resp::{
        cmd, BulkString, Command, CommandArgs, FromKeyValueValueArray, FromSingleValueArray,
        FromValue, IntoArgs, Value,
    },
    ClientTrait, Error, Future, GraphCache, GraphValue, PipelinePreparedCommand, PreparedCommand,
    Result,
};
use smallvec::SmallVec;
use std::{
    collections::HashMap,
    future,
    str::{from_utf8, FromStr},
};

/// A group of Redis commands related to [`RedisGraph`](https://redis.io/docs/stack/graph/)
///
/// # See Also
/// [RedisGraph Commands](https://redis.io/commands/?group=graph)
pub trait GraphCommands {
    /// Retrieves the current value of a RedisGraph configuration parameter.
    ///
    /// # Arguments
    /// * `name` - name of the configuration parameter, or '*' for all.
    ///
    /// # Return
    /// Key/value collection holding names & values of the requested configs
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.config-get/>](https://redis.io/commands/graph.config-get/)
    /// * [`Configuration Parameters`](https://redis.io/docs/stack/graph/configuration/)
    #[must_use]
    fn graph_config_get<N, V, R>(&mut self, name: impl Into<BulkString>) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        N: FromValue,
        V: FromValue,
        R: FromKeyValueValueArray<N, V>,
    {
        prepare_command(self, cmd("GRAPH.CONFIG").arg("GET").arg(name))
    }

    /// Set the value of a RedisGraph configuration parameter.
    ///
    /// # Arguments
    /// * `name` - name of the configuration option.
    /// * `value` - value of the configuration option.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.config-set/>](https://redis.io/commands/graph.config-set/)
    /// * [`Configuration Parameters`](https://redis.io/docs/stack/graph/configuration/)
    ///
    /// # Note
    /// As detailed in the link above, not all RedisGraph configuration parameters can be set at run-time.
    #[must_use]
    fn graph_config_set(
        &mut self,
        name: impl Into<BulkString>,
        value: impl Into<BulkString>,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.CONFIG").arg("SET").arg(name).arg(value))
    }

    /// Completely removes the graph and all of its entities.
    ///
    /// # Arguments
    /// * `graph` - name of the graph to delete.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.delete/>](https://redis.io/commands/graph.delete/)
    #[must_use]
    fn graph_delete(&mut self, graph: impl Into<BulkString>) -> PreparedCommand<Self, String>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.DELETE").arg(graph))
    }

    /// Constructs a query execution plan but does not run it.
    ///
    /// Inspect this execution plan to better understand how your query will get executed.
    ///
    /// # Arguments
    /// * `graph` - graph name.
    /// * `query` - query to explain.
    ///
    /// # Return
    /// String representation of a query execution plan
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.explain/>](https://redis.io/commands/graph.explain/)
    #[must_use]
    fn graph_explain<R: FromValue, RR: FromSingleValueArray<R>>(
        &mut self,
        graph: impl Into<BulkString>,
        query: impl Into<BulkString>,
    ) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.EXPLAIN").arg(graph).arg(query))
    }

    /// Lists all graph keys in the keyspace.
    ///
    /// # Return
    /// String collection of graph names
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.list/>](https://redis.io/commands/graph.list/)
    #[must_use]
    fn graph_list<R: FromValue, RR: FromSingleValueArray<R>>(&mut self) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.LIST"))
    }

    /// Executes a query and produces an execution plan augmented with metrics for each operation's execution.
    ///
    /// # Arguments
    /// * `graph` - graph name.
    /// * `query`- query to profile
    /// * `options` - See [`GraphQueryOptions`](crate::GraphQueryOptions)
    ///
    /// # Return
    /// String representation of a query execution plan, with details on results produced by and time spent in each operation.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.list/>](https://redis.io/commands/graph.list/)
    #[must_use]
    fn graph_profile<R: FromValue, RR: FromSingleValueArray<R>>(
        &mut self,
        graph: impl Into<BulkString>,
        query: impl Into<BulkString>,
        options: GraphQueryOptions,
    ) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.LIST").arg(graph).arg(query).arg(options))
    }

    /// Executes the given query against a specified graph.
    ///
    /// # Arguments
    /// * `graph` - graph name.
    /// * `query`- query to execute
    /// * `options` - See [`GraphQueryOptions`](crate::GraphQueryOptions)
    ///
    /// # Return
    /// returns a [`result set`](GraphResultSet)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.query/>](https://redis.io/commands/graph.query/)
    /// * [`openCypher query language`](https://opencypher.org/)
    #[must_use]
    fn graph_query(
        &mut self,
        graph: impl Into<BulkString>,
        query: impl Into<BulkString>,
        options: GraphQueryOptions,
    ) -> PreparedCommand<Self, GraphResultSet>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("GRAPH.QUERY")
                .arg(graph)
                .arg(query)
                .arg(options)
                .arg("--compact"),
        )
        .post_process(Box::new(GraphResultSet::post_process))
    }

    /// Executes a given read only query against a specified graph
    ///
    /// # Arguments
    /// * `graph` - graph name.
    /// * `query`- query to execute
    /// * `options` - See [`GraphQueryOptions`](crate::GraphQueryOptions)
    ///
    /// # Return
    /// returns a [`result set`](GraphResultSet)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.ro_query/>](https://redis.io/commands/graph.ro_query/)
    #[must_use]
    fn graph_ro_query(
        &mut self,
        graph: impl Into<BulkString>,
        query: impl Into<BulkString>,
        options: GraphQueryOptions,
    ) -> PreparedCommand<Self, GraphResultSet>
    where
        Self: Sized,
    {
        prepare_command(
            self,
            cmd("GRAPH.RO_QUERY")
                .arg(graph)
                .arg(query)
                .arg(options)
                .arg("--compact"),
        )
        .post_process(Box::new(GraphResultSet::post_process))
    }

    /// Returns a list containing up to 10 of the slowest queries issued against the given graph ID.
    ///
    /// # Arguments
    /// * `graph` - graph name.
    ///
    /// # Return
    /// A collection of GraphSlowlogResult
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.slowlog/>](https://redis.io/commands/graph.slowlog/)
    #[must_use]
    fn graph_slowlog<R: FromSingleValueArray<GraphSlowlogResult>>(
        &mut self,
        graph: impl Into<BulkString>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.SLOWLOG").arg(graph))
    }
}

/// Options for the [`graph_query`](crate::GraphCommands::graph_query) command
#[derive(Default)]
pub struct GraphQueryOptions {
    command_args: CommandArgs,
}

impl GraphQueryOptions {
    /// Timeout for the query in milliseconds
    #[must_use]
    pub fn timeout(timeout: u64) -> Self {
        Self {
            command_args: CommandArgs::Empty.arg("TIMEOUT").arg(timeout),
        }
    }
}

impl IntoArgs for GraphQueryOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result set for the [`graph_query`](crate::GraphCommands::graph_query) command
#[derive(Debug)]
pub struct GraphResultSet {
    pub header: GraphHeader,
    pub rows: Vec<GraphResultRow>,
    pub statistics: GraphQueryStatistics,
}

impl GraphResultSet {
    pub(crate) fn post_process(
        value: Value,
        command: Command,
        client: &mut dyn ClientTrait,
    ) -> Future<Self> {
        let Some(BulkString::Str(graph_name)) = command.args.iter().next() else {
            return Box::pin(future::ready(Err(Error::Client("Cannot parse graph command".to_owned()))));
        };
        Self::from_value_async(value, graph_name, client)
    }

    pub(crate) fn from_value_async<'a, 'b: 'a>(
        value: Value,
        graph_name: &'b str,
        client: &'a mut dyn ClientTrait,
    ) -> Future<'a, Self> {
        Box::pin(async move {
            let mut cache = client
                .get_cache()
                .get_entry::<GraphCache>(&format!("graph:{graph_name}"))?;

            if !cache.check_for_result(&value) {
                let num_node_labels = cache.node_labels.len();
                let num_prop_keys = cache.property_keys.len();
                let num_rel_types = cache.relationship_types.len();

                let (node_labels, prop_keys, rel_types) = Self::load_missing_ids(
                    graph_name,
                    client,
                    num_node_labels,
                    num_prop_keys,
                    num_rel_types,
                )
                .await?;

                cache = client
                    .get_cache()
                    .get_entry::<GraphCache>(&format!("graph:{graph_name}"))?;

                cache.update(node_labels, prop_keys, rel_types);

                log::debug!("cache updated: {cache:?}");
            }

            let values: Vec<Value> = value.into()?;
            let mut iter = values.into_iter();

            match (iter.next(), iter.next(), iter.next(), iter.next()) {
                (Some(statistics), None, None, None) => Ok(Self {
                    header: Default::default(),
                    rows: Default::default(),
                    statistics: statistics.into()?,
                }),
                (Some(header), Some(Value::Array(Some(rows))), Some(statistics), None) => {
                    let rows = rows
                        .into_iter()
                        .map(|v| GraphResultRow::from_value(v, cache))
                        .collect::<Result<Vec<GraphResultRow>>>()?;

                    Ok(Self {
                        header: header.into()?,
                        rows,
                        statistics: statistics.into()?,
                    })
                }
                _ => Err(Error::Client("Cannot parse GraphStatistics".to_owned())),
            }
        })
    }

    async fn load_missing_ids(
        graph_name: &str,
        client: &mut dyn ClientTrait,
        num_node_labels: usize,
        num_prop_keys: usize,
        num_rel_types: usize,
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let mut pipeline = client.create_pipeline();

        // node labels
        pipeline
            .graph_query(
                graph_name.to_owned(),
                format!(
                    "CALL db.labels() YIELD label RETURN label SKIP {}",
                    num_node_labels
                ),
                GraphQueryOptions::default(),
            )
            .queue();

        // property keys
        pipeline
            .graph_query(
                graph_name.to_owned(),
                format!(
                    "CALL db.propertyKeys() YIELD propertyKey RETURN propertyKey SKIP {}",
                    num_prop_keys
                ),
                GraphQueryOptions::default(),
            )
            .queue();

        // relationship types
        pipeline
            .graph_query(
                graph_name.to_owned(),
                format!(
                    "CALL db.relationshipTypes() YIELD relationshipType RETURN relationshipType SKIP {}",
                    num_rel_types
                ),
                GraphQueryOptions::default(),
            )
            .queue();

        let result: Value = pipeline.execute().await?;

        let Value::Array(Some(results)) = result else {
            return Err(Error::Client("Cannot parse GraphResultSet from result".to_owned()));
        };

        let mut iter = results.into_iter();

        let (Some(node_labels), Some(prop_keys), Some(rel_types)) = (iter.next(), iter.next(), iter.next()) else {
            return Err(Error::Client("Cannot parse GraphResultSet from result".to_owned()))
        };

        let node_labels = GraphResultSet::from_value_async(node_labels, graph_name, client).await?;
        let prop_keys = GraphResultSet::from_value_async(prop_keys, graph_name, client).await?;
        let rel_types = GraphResultSet::from_value_async(rel_types, graph_name, client).await?;

        let node_labels = node_labels
            .rows
            .into_iter()
            .map(|mut r| r.values.pop().unwrap().into::<String>())
            .collect::<Result<Vec<String>>>()?;

        let prop_keys = prop_keys
            .rows
            .into_iter()
            .map(|mut r| r.values.pop().unwrap().into::<String>())
            .collect::<Result<Vec<String>>>()?;

        let rel_types = rel_types
            .rows
            .into_iter()
            .map(|mut r| r.values.pop().unwrap().into::<String>())
            .collect::<Result<Vec<String>>>()?;

        Ok((node_labels, prop_keys, rel_types))
    }
}

impl FromValue for GraphResultSet {
    fn from_value(_value: Value) -> Result<Self> {
        unimplemented!();
    }
}

/// Header part of a graph ['result set`](GraphResultSet)
#[derive(Debug, Default)]
pub struct GraphHeader {
    pub column_names: Vec<String>,
}

impl FromValue for GraphHeader {
    fn from_value(value: Value) -> Result<Self> {
        let header: SmallVec<[(u8, String); 10]> = value.into()?;
        let column_names = header
            .into_iter()
            .map(|(_colmun_type, column_name)| column_name)
            .collect();

        Ok(Self { column_names })
    }
}

#[derive(Debug)]
pub struct GraphResultRow {
    /// collection of values
    ///
    /// each value matches a column name in the result set [`header`](GraphHeader)
    pub values: Vec<GraphValue>,
}

impl GraphResultRow {
    pub(crate) fn from_value(value: Value, cache: &GraphCache) -> Result<Self> {
        let Value::Array(Some(values)) = value else {
            return Err(Error::Client("Cannot parse GraphResultRow".to_owned()));
        };

        Ok(Self {
            values: values
                .into_iter()
                .map(|v| GraphValue::from_value(v, cache))
                .collect::<Result<Vec<GraphValue>>>()?,
        })
    }
}

/// Statistics part of a graph ['result set`](crate::GraphResultSet)
#[derive(Debug)]
pub struct GraphQueryStatistics {
    pub labels_added: usize,
    pub labels_removed: usize,
    pub nodes_created: usize,
    pub nodes_deleted: usize,
    pub properties_set: usize,
    pub properties_removed: usize,
    pub relationships_created: usize,
    pub relationships_deleted: usize,
    pub indices_created: usize,
    pub indices_deleted: usize,
    pub cached_execution: bool,
    pub query_internal_execution_time: f64,
    pub additional_statistics: HashMap<String, String>,
}

impl FromValue for GraphQueryStatistics {
    fn from_value(value: Value) -> Result<Self> {
        fn remove_and_parse<F: FromStr + Default>(
            map: &mut HashMap<String, String>,
            key: &str,
        ) -> Result<F> {
            match map.remove(key) {
                Some(value) => match value.parse::<F>() {
                    Ok(value) => Ok(value),
                    Err(_) => Err(Error::Client(
                        "Cannot parse GraphQueryStatistics".to_owned(),
                    )),
                },
                None => Ok(F::default()),
            }
        }

        fn remove_and_parse_query_execution_time(map: &mut HashMap<String, String>) -> Result<f64> {
            match map.remove("Query internal execution time") {
                Some(value) => {
                    let Some((value, _milliseconds))= value.split_once(' ') else {
                        return Err(Error::Client("Cannot parse GraphQueryStatistics".to_owned()));
                    };

                    match value.parse::<f64>() {
                        Ok(value) => Ok(value),
                        Err(_) => Err(Error::Client(
                            "Cannot parse GraphQueryStatistics".to_owned(),
                        )),
                    }
                }
                None => Ok(0f64),
            }
        }

        let values: Vec<Value> = value.into()?;
        let mut statistics: HashMap<String, String> = values
            .into_iter()
            .map(|v| {
                let Value::BulkString(BulkString::Binary(s)) = v else {
                    return Err(Error::Client("Cannot parse GraphQueryStatistics".to_owned()));
                };

                let str = from_utf8(&s)?;
                let Some((name, value))= str.split_once(": ") else {
                    return Err(Error::Client("Cannot parse GraphQueryStatistics".to_owned()));
                };

                Ok((name.to_owned(), value.to_owned()))
            })
            .collect::<Result<HashMap<String, String>>>()?;

        Ok(Self {
            labels_added: remove_and_parse(&mut statistics, "Labels added")?,
            labels_removed: remove_and_parse(&mut statistics, "Labels removed")?,
            nodes_created: remove_and_parse(&mut statistics, "Nodes created")?,
            nodes_deleted: remove_and_parse(&mut statistics, "Nodes deleted:")?,
            properties_set: remove_and_parse(&mut statistics, "Properties set")?,
            properties_removed: remove_and_parse(&mut statistics, "Properties removed")?,
            relationships_created: remove_and_parse(&mut statistics, "Relationships created")?,
            relationships_deleted: remove_and_parse(&mut statistics, "Relationships deleted")?,
            indices_created: remove_and_parse(&mut statistics, "Indices created")?,
            indices_deleted: remove_and_parse(&mut statistics, "Indices deleted")?,
            cached_execution: remove_and_parse::<u8>(&mut statistics, "Cached execution")? != 0,
            query_internal_execution_time: remove_and_parse_query_execution_time(&mut statistics)?,
            additional_statistics: statistics,
        })
    }
}

/// Result for the [`graph_slowlog`](crate::GraphCommands::graph_slowlog) command
#[derive(Debug)]
pub struct GraphSlowlogResult {
    /// A Unix timestamp at which the log entry was processed.
    pub processing_time: u64,
    /// The issued command.
    pub issued_command: String,
    /// The issued query.
    pub issued_query: String,
    /// The amount of time needed for its execution, in milliseconds.
    pub execution_duration: f64,
}

impl FromValue for GraphSlowlogResult {
    fn from_value(value: Value) -> Result<Self> {
        let (processing_time, issued_command, issued_query, execution_duration) =
            value.into::<(u64, String, String, f64)>()?;

        Ok(Self {
            processing_time,
            issued_command,
            issued_query,
            execution_duration,
        })
    }
}
