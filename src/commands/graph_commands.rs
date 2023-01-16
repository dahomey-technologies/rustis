use crate::{
    client::{prepare_command, Client, PreparedCommand},
    commands::{GraphCache, GraphValue, GraphValueArraySeed},
    resp::{
        cmd, Command, CommandArg, CommandArgs, FromKeyValueArray, FromSingleValue, FromValue,
        FromValueArray, IntoArgs, SingleArg, Value,
    },
    Error, Future, Result,
};
use serde::{
    de::{self, DeserializeOwned, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use smallvec::SmallVec;
use std::{
    collections::HashMap,
    fmt, future,
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
    fn graph_config_get<N, V, R>(&mut self, name: impl SingleArg) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        N: FromSingleValue,
        V: FromSingleValue,
        R: FromKeyValueArray<N, V>,
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
        name: impl SingleArg,
        value: impl SingleArg,
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
    fn graph_delete(&mut self, graph: impl SingleArg) -> PreparedCommand<Self, String>
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
    fn graph_explain<R: FromSingleValue + DeserializeOwned, RR: FromValueArray<R>>(
        &mut self,
        graph: impl SingleArg,
        query: impl SingleArg,
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
    fn graph_list<R: FromSingleValue + DeserializeOwned, RR: FromValueArray<R>>(
        &mut self,
    ) -> PreparedCommand<Self, RR>
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
    /// * `options` - See [`GraphQueryOptions`](GraphQueryOptions)
    ///
    /// # Return
    /// String representation of a query execution plan, with details on results produced by and time spent in each operation.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.list/>](https://redis.io/commands/graph.list/)
    #[must_use]
    fn graph_profile<R: FromSingleValue + DeserializeOwned, RR: FromValueArray<R>>(
        &mut self,
        graph: impl SingleArg,
        query: impl SingleArg,
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
    /// * `options` - See [`GraphQueryOptions`](GraphQueryOptions)
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
        graph: impl SingleArg,
        query: impl SingleArg,
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
    /// * `options` - See [`GraphQueryOptions`](GraphQueryOptions)
    ///
    /// # Return
    /// returns a [`result set`](GraphResultSet)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/graph.ro_query/>](https://redis.io/commands/graph.ro_query/)
    #[must_use]
    fn graph_ro_query(
        &mut self,
        graph: impl SingleArg,
        query: impl SingleArg,
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
    fn graph_slowlog<R: FromValueArray<GraphSlowlogResult>>(
        &mut self,
        graph: impl SingleArg,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("GRAPH.SLOWLOG").arg(graph))
    }
}

/// Options for the [`graph_query`](GraphCommands::graph_query) command
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

/// Result set for the [`graph_query`](GraphCommands::graph_query) command
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
        client: &mut Client,
    ) -> Future<Self> {
        let Some(CommandArg::Str(graph_name)) = command.args.iter().next() else {
            return Box::pin(future::ready(Err(Error::Client("Cannot parse graph command".to_owned()))));
        };
        Self::from_value_async(value, graph_name, client)
    }

    pub(crate) fn from_value_async<'a, 'b: 'a>(
        value: Value,
        graph_name: &'b str,
        client: &'a mut Client,
    ) -> Future<'a, Self> {
        Box::pin(async move {
            let cache_key = format!("graph:{graph_name}");
            let (cache_hit, num_node_labels, num_prop_keys, num_rel_types) = {
                let client_state = client.get_client_state();
                match client_state.get_state::<GraphCache>(&cache_key)? {
                    Some(cache) => {
                        if cache.check_for_result(&value) {
                            (true, 0, 0, 0)
                        } else {
                            (
                                false,
                                cache.node_labels.len(),
                                cache.property_keys.len(),
                                cache.relationship_types.len(),
                            )
                        }
                    }
                    None => {
                        let cache = GraphCache::default();

                        if cache.check_for_result(&value) {
                            (true, 0, 0, 0)
                        } else {
                            (false, 0, 0, 0)
                        }
                    }
                }
            };

            if !cache_hit {
                let (node_labels, prop_keys, rel_types) = Self::load_missing_ids(
                    graph_name,
                    client,
                    num_node_labels,
                    num_prop_keys,
                    num_rel_types,
                )
                .await?;

                let mut client_state = client.get_client_state_mut();
                let cache = client_state.get_state_mut::<GraphCache>(&cache_key)?;

                cache.update(
                    num_node_labels,
                    num_prop_keys,
                    num_rel_types,
                    node_labels,
                    prop_keys,
                    rel_types,
                );

                log::debug!("cache updated: {cache:?}");
            } else if num_node_labels == 0 && num_prop_keys == 0 && num_rel_types == 0 {
                // force cache creation
                let mut client_state = client.get_client_state_mut();
                client_state.get_state_mut::<GraphCache>(&cache_key)?;

                log::debug!("graph cache created");
            }

            let values: Vec<Value> = value.into()?;
            let mut iter = values.into_iter();

            match (iter.next(), iter.next(), iter.next(), iter.next()) {
                (Some(statistics), None, None, None) => Ok(Self {
                    header: Default::default(),
                    rows: Default::default(),
                    statistics: statistics.into()?,
                }),
                (Some(header), Some(Value::Array(rows)), Some(statistics), None) => {
                    let client_state = client.get_client_state();
                    let Some(cache) = client_state.get_state::<GraphCache>(&cache_key)? else {
                        return Err(Error::Client("Cannot find graph cache".to_owned()));
                    };

                    let rows = rows
                        .into_iter()
                        .map(|v| GraphResultRowSeed { cache }.deserialize(&v))
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
        client: &mut Client,
        num_node_labels: usize,
        num_prop_keys: usize,
        num_rel_types: usize,
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let mut pipeline = client.create_pipeline();

        // node labels
        pipeline.queue(cmd("GRAPH.QUERY").arg(graph_name.to_owned()).arg(format!(
            "CALL db.labels() YIELD label RETURN label SKIP {}",
            num_node_labels
        )));

        // property keys
        pipeline.queue(cmd("GRAPH.QUERY").arg(graph_name.to_owned()).arg(format!(
            "CALL db.propertyKeys() YIELD propertyKey RETURN propertyKey SKIP {}",
            num_prop_keys
        )));

        // relationship types
        pipeline.queue(cmd("GRAPH.QUERY").arg(graph_name.to_owned()).arg(format!(
            "CALL db.relationshipTypes() YIELD relationshipType RETURN relationshipType SKIP {}",
            num_rel_types
        )));

        let (MappingsResult(node_labels), MappingsResult(prop_keys), MappingsResult(rel_types)) =
            pipeline
                .execute::<(MappingsResult, MappingsResult, MappingsResult)>()
                .await?;

        Ok((node_labels, prop_keys, rel_types))
    }
}

impl FromValue for GraphResultSet {
    fn from_value(_value: Value) -> Result<Self> {
        unimplemented!();
    }
}

/// Result for Mappings
/// See: https://redis.io/docs/stack/graph/design/client_spec/#procedure-calls
struct MappingsResult(Vec<String>);

impl<'de> Deserialize<'de> for MappingsResult {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MappingsSeed;

        impl<'de> DeserializeSeed<'de> for MappingsSeed {
            type Value = Vec<String>;

            #[inline]
            fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct MappingSeed;

                impl<'de> DeserializeSeed<'de> for MappingSeed {
                    type Value = String;

                    #[inline]
                    fn deserialize<D>(
                        self,
                        deserializer: D,
                    ) -> std::result::Result<Self::Value, D::Error>
                    where
                        D: Deserializer<'de>,
                    {
                        struct MappingVisitor;

                        impl<'de> Visitor<'de> for MappingVisitor {
                            type Value = String;

                            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                                formatter.write_str("String")
                            }

                            fn visit_seq<A>(
                                self,
                                mut seq: A,
                            ) -> std::result::Result<Self::Value, A::Error>
                            where
                                A: de::SeqAccess<'de>,
                            {
                                let Some(mapping) = seq.next_element::<String>()? else {
                                    return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
                                };

                                Ok(mapping)
                            }
                        }

                        deserializer.deserialize_seq(MappingVisitor)
                    }
                }

                struct MappingsVisitor;

                impl<'de> Visitor<'de> for MappingsVisitor {
                    type Value = Vec<String>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("Vec<String>")
                    }

                    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
                    where
                        A: de::SeqAccess<'de>,
                    {
                        let mut mappings = if let Some(size_hint) = seq.size_hint() {
                            Vec::with_capacity(size_hint)
                        } else {
                            Vec::new()
                        };

                        while let Some(mapping) = seq.next_element_seed(MappingSeed)? {
                            mappings.push(mapping);
                        }

                        Ok(mappings)
                    }
                }

                deserializer.deserialize_seq(MappingsVisitor)
            }
        }

        struct MappingsResultVisitor;

        impl<'de> Visitor<'de> for MappingsResultVisitor {
            type Value = MappingsResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("MappingsResult")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let Some(_header) = seq.next_element::<Vec::<String>>()? else {
                    return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
                };

                let Some(mappings) = seq.next_element_seed(MappingsSeed)? else {
                    return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
                };

                let Some(_stats) = seq.next_element::<Vec::<String>>()? else {
                    return Err(de::Error::invalid_length(2, &"fewer elements in sequence"));
                };

                Ok(MappingsResult(mappings))
            }
        }

        deserializer.deserialize_seq(MappingsResultVisitor)
    }
}

/// Header part of a graph ['result set`](GraphResultSet)
#[derive(Debug, Default)]
pub struct GraphHeader {
    pub column_names: Vec<String>,
}

impl<'de> Deserialize<'de> for GraphHeader {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let header = SmallVec::<[(u16, String); 10]>::deserialize(deserializer)?;
        let column_names = header
            .into_iter()
            .map(|(_colmun_type, column_name)| column_name)
            .collect();

        Ok(Self { column_names })
    }
}

/// Result row for the [`graph_query`](GraphCommands::graph_query) command
#[derive(Debug)]
pub struct GraphResultRow {
    /// collection of values
    ///
    /// each value matches a column name in the result set [`header`](GraphHeader)
    pub values: Vec<GraphValue>,
}

pub struct GraphResultRowSeed<'a> {
    cache: &'a GraphCache,
}

impl<'de, 'a> DeserializeSeed<'de> for GraphResultRowSeed<'a> {
    type Value = GraphResultRow;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = GraphValueArraySeed { cache: self.cache }.deserialize(deserializer)?;

        Ok(GraphResultRow { values })
    }
}

/// Statistics part of a graph ['result set`](GraphResultSet)
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
                let Value::BulkString(s) = v else {
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

/// Result for the [`graph_slowlog`](GraphCommands::graph_slowlog) command
#[derive(Debug, Deserialize)]
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
