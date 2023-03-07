use crate::{
    client::{prepare_command, Client, PreparedCommand},
    commands::{GraphCache, GraphValue, GraphValueArraySeed},
    resp::{
        cmd, Command, CommandArg, CommandArgs, KeyValueCollectionResponse, PrimitiveResponse, CollectionResponse,
        IntoArgs, RespBuf, RespDeserializer, SingleArg,
    },
    Error, Future, Result,
};
use serde::{
    de::{self, DeserializeOwned, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use smallvec::SmallVec;
use std::{collections::HashMap, fmt, future, str::FromStr};

/// A group of Redis commands related to [`RedisGraph`](https://redis.io/docs/stack/graph/)
///
/// # See Also
/// [RedisGraph Commands](https://redis.io/commands/?group=graph)
pub trait GraphCommands<'a> {
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
    fn graph_config_get<N, V, R>(self, name: impl SingleArg) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        N: PrimitiveResponse,
        V: PrimitiveResponse,
        R: KeyValueCollectionResponse<N, V>,
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
        self,
        name: impl SingleArg,
        value: impl SingleArg,
    ) -> PreparedCommand<'a, Self, ()>
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
    fn graph_delete(self, graph: impl SingleArg) -> PreparedCommand<'a, Self, String>
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
    fn graph_explain<R: PrimitiveResponse + DeserializeOwned, RR: CollectionResponse<R>>(
        self,
        graph: impl SingleArg,
        query: impl SingleArg,
    ) -> PreparedCommand<'a, Self, RR>
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
    fn graph_list<R: PrimitiveResponse + DeserializeOwned, RR: CollectionResponse<R>>(
        self,
    ) -> PreparedCommand<'a, Self, RR>
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
    fn graph_profile<R: PrimitiveResponse + DeserializeOwned, RR: CollectionResponse<R>>(
        self,
        graph: impl SingleArg,
        query: impl SingleArg,
        options: GraphQueryOptions,
    ) -> PreparedCommand<'a, Self, RR>
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
        self,
        graph: impl SingleArg,
        query: impl SingleArg,
        options: GraphQueryOptions,
    ) -> PreparedCommand<'a, Self, GraphResultSet>
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
        .custom_converter(Box::new(GraphResultSet::custom_conversion))
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
        self,
        graph: impl SingleArg,
        query: impl SingleArg,
        options: GraphQueryOptions,
    ) -> PreparedCommand<'a, Self, GraphResultSet>
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
        .custom_converter(Box::new(GraphResultSet::custom_conversion))
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
    fn graph_slowlog<R: CollectionResponse<GraphSlowlogResult>>(
        self,
        graph: impl SingleArg,
    ) -> PreparedCommand<'a, Self, R>
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
#[derive(Debug, Deserialize)]
pub struct GraphResultSet {
    pub header: GraphHeader,
    pub rows: Vec<GraphResultRow>,
    pub statistics: GraphQueryStatistics,
}

impl GraphResultSet {
    pub(crate) fn custom_conversion(
        resp_buffer: RespBuf,
        command: Command,
        client: &Client,
    ) -> Future<Self> {
        let Some(CommandArg::Str(graph_name)) = command.args.iter().next() else {
            return Box::pin(future::ready(Err(Error::Client("Cannot parse graph command".to_owned()))));
        };

        let graph_name = *graph_name;

        Box::pin(async move {
            let cache_key = format!("graph:{graph_name}");
            let (cache_hit, num_node_labels, num_prop_keys, num_rel_types) = {
                let client_state = client.get_client_state();
                match client_state.get_state::<GraphCache>(&cache_key)? {
                    Some(cache) => {
                        let mut deserializer = RespDeserializer::new(&resp_buffer);
                        if cache.check_for_result(&mut deserializer)? {
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
                        let mut deserializer = RespDeserializer::new(&resp_buffer);
                        if cache.check_for_result(&mut deserializer)? {
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

            let mut deserializer = RespDeserializer::new(&resp_buffer);
            Self::deserialize(&mut deserializer, client, &cache_key)
        })
    }

    fn deserialize<'de, D>(
        deserializer: D,
        client: &Client,
        cache_key: &str,
    ) -> std::result::Result<GraphResultSet, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct GraphResultSetVisitor<'a, 'b> {
            client: &'a Client,
            cache_key: &'b str,
        }

        impl<'a, 'b, 'de> Visitor<'de> for GraphResultSetVisitor<'a, 'b> {
            type Value = GraphResultSet;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("GraphResultSet")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(size) = seq.size_hint() else {
                    return Err(de::Error::custom("size hint is mandatory for GraphResultSet"));
                };

                if size == 1 {
                    let Some(statistics) = seq.next_element::<GraphQueryStatistics>()? else {
                        return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                    };

                    Ok(GraphResultSet {
                        header: Default::default(),
                        rows: Default::default(),
                        statistics,
                    })
                } else {
                    let Some(header) = seq.next_element::<GraphHeader>()? else {
                        return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                    };

                    let client_state = self.client.get_client_state();
                    let Ok(Some(cache)) = client_state.get_state::<GraphCache>(self.cache_key) else {
                        return Err(de::Error::custom("Cannot find graph cache"));
                    };

                    let Some(rows) = seq.next_element_seed(GraphResultRowsSeed { cache })? else {
                        return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                    };

                    let Some(statistics) = seq.next_element::<GraphQueryStatistics>()? else {
                        return Err(de::Error::invalid_length(2, &"more elements in sequence"));
                    };

                    Ok(GraphResultSet {
                        header,
                        rows,
                        statistics,
                    })
                }
            }
        }

        deserializer.deserialize_seq(GraphResultSetVisitor { client, cache_key })
    }

    async fn load_missing_ids(
        graph_name: &str,
        client: &Client,
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
                                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
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
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let Some(mappings) = seq.next_element_seed(MappingsSeed)? else {
                    return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                };

                let Some(_stats) = seq.next_element::<Vec::<String>>()? else {
                    return Err(de::Error::invalid_length(2, &"more elements in sequence"));
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
#[derive(Debug, Deserialize)]
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

struct GraphResultRowsSeed<'a> {
    cache: &'a GraphCache,
}

impl<'de, 'a> Visitor<'de> for GraphResultRowsSeed<'a> {
    type Value = Vec<GraphResultRow>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Vec<GraphResultRow>")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut rows = if let Some(size) = seq.size_hint() {
            Vec::with_capacity(size)
        } else {
            Vec::new()
        };

        while let Some(row) = seq.next_element_seed(GraphResultRowSeed { cache: self.cache })? {
            rows.push(row);
        }

        Ok(rows)
    }
}

impl<'de, 'a> DeserializeSeed<'de> for GraphResultRowsSeed<'a> {
    type Value = Vec<GraphResultRow>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

/// Statistics part of a graph ['result set`](GraphResultSet)
#[derive(Debug, Default)]
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

impl<'de> Deserialize<'de> for GraphQueryStatistics {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct GraphQueryStatisticsVisitor;

        impl<'de> Visitor<'de> for GraphQueryStatisticsVisitor {
            type Value = GraphQueryStatistics;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("GraphQueryStatistics")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                fn parse<'de, A, F>(value: &str) -> std::result::Result<F, A::Error>
                where
                    A: de::SeqAccess<'de>,
                    F: FromStr,
                {
                    match value.parse::<F>() {
                        Ok(value) => Ok(value),
                        Err(_) => Err(de::Error::custom(format!(
                            "Cannot parse GraphQueryStatistics: {value}"
                        ))),
                    }
                }

                fn parse_query_execution_time<'de, A>(
                    value: &str,
                ) -> std::result::Result<f64, A::Error>
                where
                    A: de::SeqAccess<'de>,
                {
                    let Some((value, _milliseconds))= value.split_once(' ') else {
                        return Err(de::Error::custom("Cannot parse GraphQueryStatistics (query exuction time)"));
                    };

                    match value.parse::<f64>() {
                        Ok(value) => Ok(value),
                        Err(_) => Err(de::Error::custom(
                            "Cannot parse GraphQueryStatistics (query exuction time)",
                        )),
                    }
                }

                let mut stats = GraphQueryStatistics::default();

                while let Some(str) = seq.next_element::<&str>()? {
                    let Some((name, value))= str.split_once(": ") else {
                        return Err(de::Error::custom("Cannot parse GraphQueryStatistics"));
                    };

                    match name {
                        "Labels added" => stats.labels_added = parse::<A, _>(value)?,
                        "Labels removed" => stats.labels_removed = parse::<A, _>(value)?,
                        "Nodes created" => stats.nodes_created = parse::<A, _>(value)?,
                        "Nodes deleted:" => stats.nodes_deleted = parse::<A, _>(value)?,
                        "Properties set" => stats.properties_set = parse::<A, _>(value)?,
                        "Properties removed" => stats.properties_removed = parse::<A, _>(value)?,
                        "Relationships created" => {
                            stats.relationships_created = parse::<A, _>(value)?
                        }
                        "Relationships deleted" => {
                            stats.relationships_deleted = parse::<A, _>(value)?
                        }
                        "Indices created" => stats.indices_created = parse::<A, _>(value)?,
                        "Indices deleted" => stats.indices_deleted = parse::<A, _>(value)?,
                        "Cached execution" => stats.cached_execution = parse::<A, u8>(value)? != 0,
                        "Query internal execution time" => {
                            stats.query_internal_execution_time =
                                parse_query_execution_time::<A>(value)?
                        }
                        _ => {
                            stats
                                .additional_statistics
                                .insert(name.to_owned(), value.to_owned());
                        }
                    }
                }

                Ok(stats)
            }
        }

        deserializer.deserialize_seq(GraphQueryStatisticsVisitor)
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
