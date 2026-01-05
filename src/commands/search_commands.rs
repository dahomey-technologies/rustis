use crate::{
    client::{PreparedCommand, prepare_command},
    commands::{GeoUnit, SortOrder},
    resp::{
        Command, RespDeserializer, Response, Value, cmd, serialize_flag, serialize_slice_with_len,
    },
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, DeserializeSeed, Visitor, value::MapAccessDeserializer},
    ser::SerializeSeq,
};
use smallvec::SmallVec;
use std::{collections::HashMap, fmt, future};

/// A group of Redis commands related to [`RedisSearch`](https://redis.io/docs/stack/search/)
///
/// # See Also
/// * [RedisSearch Commands](https://redis.io/commands/?group=search)
/// * [Auto-Suggest Commands](https://redis.io/commands/?group=suggestion)
pub trait SearchCommands<'a>: Sized {
    /// Run a search query on an index,
    /// and perform aggregate transformations on the results,
    /// extracting statistics etc from them
    ///
    /// # Arguments
    /// * `index` - index against which the query is executed.
    /// * `query`- is base filtering query that retrieves the documents.\
    ///   It follows the exact same syntax as the search query,\
    ///   including filters, unions, not, optional, and so on.
    /// * `options` - See [`FtAggregateOptions`](FtAggregateOptions)
    ///
    /// # Returns
    /// An instance of [`FtAggregateResult`](FtAggregateResult)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.aggregate/>](https://redis.io/commands/ft.aggregate/)
    /// * [`RedisSeach Aggregations`](https://redis.io/docs/stack/search/reference/aggregations/)
    #[must_use]
    fn ft_aggregate(
        self,
        index: impl Serialize,
        query: impl Serialize,
        options: FtAggregateOptions,
    ) -> PreparedCommand<'a, Self, FtAggregateResult> {
        prepare_command(self, cmd("FT.AGGREGATE").arg(index).arg(query).arg(options))
    }

    /// Add an alias to an index
    ///
    /// # Arguments
    /// * `index` - The index.
    /// * `alias` - alias to be added to an index
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.aliasadd/>](https://redis.io/commands/ft.aliasadd/)
    #[must_use]
    fn ft_aliasadd(
        self,
        alias: impl Serialize,
        index: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("FT.ALIASADD").arg(alias).arg(index))
    }

    /// Remove an alias from an index
    ///
    /// # Arguments
    /// * `alias` - alias to be removed
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.aliasdel/>](https://redis.io/commands/ft.aliasdel/)
    #[must_use]
    fn ft_aliasdel(self, alias: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("FT.ALIASDEL").arg(alias))
    }

    /// Add an alias to an index.
    ///
    /// If the alias is already associated with another index,
    /// this command removes the alias association with the previous index.
    ///
    /// # Arguments
    /// * `index` - The index.
    /// * `alias` - alias to be added to an index
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.aliasupdate/>](https://redis.io/commands/ft.aliasupdate/)
    #[must_use]
    fn ft_aliasupdate(
        self,
        alias: impl Serialize,
        index: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("FT.ALIASUPDATE").arg(alias).arg(index))
    }

    /// Add a new attribute to the index.
    ///
    /// Adding an attribute to the index causes any future document updates
    /// to use the new attribute when indexing and reindexing existing documents.
    ///
    /// # Arguments
    /// * `index` - index name to create.
    /// * `skip_initial_scan` - if set, does not scan and index.
    /// * `attribute` - attribute to add.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.alter/>](https://redis.io/commands/ft.alter/)
    #[must_use]
    fn ft_alter(
        self,
        index: impl Serialize,
        skip_initial_scan: bool,
        attribute: FtFieldSchema,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FT.ALTER")
                .arg(index)
                .arg_if(skip_initial_scan, "SKIPINITIALSCAN")
                .arg("SCHEMA")
                .arg("ADD")
                .arg(attribute),
        )
    }

    /// Retrieve configuration options
    ///
    /// # Arguments
    /// * `option` - name of the configuration option, or '*' for all.
    ///
    /// # Return
    /// Key/value collection holding names & values of the requested configs
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.config-get/>](https://redis.io/commands/ft.config-get/)
    #[must_use]
    fn ft_config_get<R: Response>(self, option: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("FT.CONFIG").arg("GET").arg(option))
    }

    /// Set configuration options
    ///
    /// # Arguments
    /// * `option` - name of the configuration option
    /// * `value` - value of the configuration option.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.config-set/>](https://redis.io/commands/ft.config-set/)
    #[must_use]
    fn ft_config_set(
        self,
        option: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("FT.CONFIG").arg("SET").arg(option).arg(value))
    }

    /// Create an index with the given specification
    ///
    /// # Arguments
    /// * `index` - Name of the index to create. If it exists, the old specification is overwritten.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.create/>](https://redis.io/commands/ft.create/)
    /// * [`Aggregations`](https://redis.io/docs/stack/search/reference/aggregations/)
    #[must_use]
    fn ft_create(
        self,
        index: impl Serialize,
        options: FtCreateOptions,
        schema: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FT.CREATE")
                .arg(index)
                .arg(options)
                .arg("SCHEMA")
                .arg(schema),
        )
    }

    /// Delete a cursor
    ///
    /// # Arguments
    /// * `index` - index name.
    /// * `cursor_id` - id of the cursor
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.cursor-del/>](https://redis.io/commands/ft.cursor-del/)
    #[must_use]
    fn ft_cursor_del(self, index: impl Serialize, cursor_id: u64) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("FT.CURSOR").arg("DEL").arg(index).arg(cursor_id))
    }

    /// Read next results from an existing cursor
    ///
    /// # Arguments
    /// * `index` - index name.
    /// * `cursor_id` - id of the cursor.
    /// * `read_size` - number of results to read. This parameter overrides
    ///   [`count`](FtWithCursorOptions::count) specified in [`ft_aggregate`](SearchCommands::ft_aggregate).
    ///
    /// # Returns
    /// an instance of [`FtAggregateResult`](FtAggregateResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.cursor-read/>](https://redis.io/commands/ft.cursor-read/)
    #[must_use]
    fn ft_cursor_read(
        self,
        index: impl Serialize,
        cursor_id: u64,
    ) -> PreparedCommand<'a, Self, FtAggregateResult> {
        prepare_command(self, cmd("FT.CURSOR").arg("READ").arg(index).arg(cursor_id))
    }

    /// Add terms to a dictionary
    ///
    /// # Arguments
    /// * `dict` - dictionary name.
    /// * `terms` - terms to add to the dictionary.
    ///
    /// # Return
    /// the number of new terms that were added.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.dictadd/>](https://redis.io/commands/ft.dictadd/)
    #[must_use]
    fn ft_dictadd(
        self,
        dict: impl Serialize,
        terms: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("FT.DICTADD").arg(dict).arg(terms))
    }

    /// Delete terms from a dictionary
    ///
    /// # Arguments
    /// * `dict` - dictionary name.
    /// * `terms` - terms to delete from the dictionary.
    ///
    /// # Return
    /// the number of terms that were deleted.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.dictdel/>](https://redis.io/commands/ft.dictdel/)
    #[must_use]
    fn ft_dictdel(
        self,
        dict: impl Serialize,
        terms: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("FT.DICTDEL").arg(dict).arg(terms))
    }

    /// Dump all terms in the given dictionary
    ///
    /// # Arguments
    /// * `dict` - dictionary name.
    ///
    /// # Return
    /// A collection, where each element is a term (bulkstring).
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.dictdump/>](https://redis.io/commands/ft.dictdump/)
    #[must_use]
    fn ft_dictdump<R: Response>(self, dict: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("FT.DICTDUMP").arg(dict))
    }

    /// Delete an index
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `dd` - drop operation that, if set, deletes the actual document hashes
    ///
    /// # Notes
    /// * By default, `ft_dropindex` does not delete the document hashes associated with the index.
    ///   Adding the `dd` option deletes the hashes as well.
    /// * When using `ft_dropindex` with the parameter `dd`, if an index creation is still running
    ///   ([`ft_create`](SearchCommands::ft_create) is running asynchronously),
    ///   only the document hashes that have already been indexed are deleted.
    ///   The document hashes left to be indexed remain in the database.
    ///   You can use [`ft_info`](SearchCommands::ft_info) to check the completion of the indexing.
    ///
    /// # Return
    /// the number of new terms that were added.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.dropindex/>](https://redis.io/commands/ft.dropindex/)
    #[must_use]
    fn ft_dropindex(self, index: impl Serialize, dd: bool) -> PreparedCommand<'a, Self, ()> {
        prepare_command(self, cmd("FT.DROPINDEX").arg(index).arg_if(dd, "DD"))
    }

    /// Return the execution plan for a complex query
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `query` - query string, as if sent to [`ft_search`](SearchCommands::ft_search).
    /// * `dialect_version` - dialect version under which to execute the query. \
    ///   If not specified, the query executes under the default dialect version set during module initial loading\
    ///   or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    ///
    /// # Notes
    /// * In the returned response, a `+` on a term is an indication of stemming.
    /// * `redis-cli --raw` to properly read line-breaks in the returned response.
    ///
    /// # Return
    /// a string representing the execution plan.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.explain/>](https://redis.io/commands/ft.explain/)
    #[must_use]
    fn ft_explain<R: Response>(
        self,
        index: impl Serialize,
        query: impl Serialize,
        dialect_version: Option<u64>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("FT.EXPLAIN").arg(index).arg(query).arg(dialect_version),
        )
    }

    /// Return the execution plan for a complex query but formatted for easier reading without using `redis-cli --raw`
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `query` - query string, as if sent to [`ft_search`](SearchCommands::ft_search).
    /// * `dialect_version` - dialect version under which to execute the query. \
    ///   If not specified, the query executes under the default dialect version set during module initial loading\
    ///   or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    ///
    /// # Notes
    /// * In the returned response, a `+` on a term is an indication of stemming.
    ///
    /// # Return
    /// a collection of strings representing the execution plan.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.explaincli/>](https://redis.io/commands/ft.explaincli/)
    #[must_use]
    fn ft_explaincli(
        self,
        index: impl Serialize,
        query: impl Serialize,
        dialect_version: Option<u64>,
    ) -> PreparedCommand<'a, Self, Value> {
        prepare_command(
            self,
            cmd("FT.EXPLAINCLI")
                .arg(index)
                .arg(query)
                .arg(dialect_version),
        )
    }

    /// Return information and statistics on the index
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    ///
    /// # Return
    /// an instance of [`FtInfoResult`](FtInfoResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.info/>](https://redis.io/commands/ft.info/)
    #[must_use]
    fn ft_info(self, index: impl Serialize) -> PreparedCommand<'a, Self, FtInfoResult> {
        prepare_command(self, cmd("FT.INFO").arg(index))
    }

    /// Returns a list of all existing indexes.
    ///
    /// # Return
    /// A collection of index names.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft._list/>](https://redis.io/commands/ft._list/)
    #[must_use]
    fn ft_list<R: Response>(self) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("FT._LIST"))
    }

    /// Perform a [`ft_search`](SearchCommands::ft_search) command and collects performance information
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `limited` - if set, removes details of reader iterator.
    /// * `query` - collection of query parameters (non including the index name)
    ///
    /// # Note
    /// To reduce the size of the output, use [`nocontent`](FtSearchOptions::nocontent) or [`limit(0,0)`](FtSearchOptions::limit) to reduce results reply
    /// or `LIMITED` to not reply with details of `reader iterators` inside builtin-unions such as `fuzzy` or `prefix`.
    ///
    /// # Return
    /// An instance of [`Value`](Value)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.profile/>](https://redis.io/commands/ft.profile/)
    #[must_use]
    fn ft_profile_search<I>(
        self,
        index: impl Serialize,
        limited: bool,
        query: impl Serialize,
    ) -> PreparedCommand<'a, Self, Value> {
        prepare_command(
            self,
            cmd("FT.PROFILE")
                .arg(index)
                .arg("SEARCH")
                .arg_if(limited, "LIMITED")
                .arg("QUERY")
                .arg(query),
        )
    }

    /// Perform a [`ft_aggregate`](SearchCommands::ft_aggregate) command and collects performance information
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `limited` - if set, removes details of reader iterator.
    /// * `query` - collection of query parameters (non including the index name)
    ///
    /// # Note
    /// To reduce the size of the output, use [`nocontent`](FtSearchOptions::nocontent) or [`limit(0,0)`](FtSearchOptions::limit) to reduce results reply
    /// or `LIMITED` to not reply with details of `reader iterators` inside builtin-unions such as `fuzzy` or `prefix`.
    ///
    /// # Return
    /// An instance of [`Value`](Value)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.profile/>](https://redis.io/commands/ft.profile/)
    #[must_use]
    fn ft_profile_aggregate(
        self,
        index: impl Serialize,
        limited: bool,
        query: impl Serialize,
    ) -> PreparedCommand<'a, Self, Value> {
        prepare_command(
            self,
            cmd("FT.PROFILE")
                .arg(index)
                .arg("AGGREGATE")
                .arg_if(limited, "LIMITED")
                .arg("QUERY")
                .arg(query),
        )
    }

    /// Search the index with a textual query, returning either documents or just ids
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `query` - text query to search. Refer to [`Query syntax`](https://redis.io/docs/stack/search/reference/query_syntax) for more details.
    /// * `options` - See [`FtSearchOptions`](FtSearchOptions)
    ///
    /// # Return
    /// An instance of [`FtSearchResult`](FtSearchResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.search/>](https://redis.io/commands/ft.search/)
    #[must_use]
    fn ft_search(
        self,
        index: impl Serialize,
        query: impl Serialize,
        options: FtSearchOptions,
    ) -> PreparedCommand<'a, Self, FtSearchResult> {
        prepare_command(self, cmd("FT.SEARCH").arg(index).arg(query).arg(options))
    }

    /// Perform spelling correction on a query, returning suggestions for misspelled terms
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `query` - search query. See [`Spellchecking`](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/spellcheck/) for more details.
    /// * `options` - See [`FtSpellCheckOptions`](FtSpellCheckOptions)
    ///
    /// # Return
    /// An instance of [`FtSpellCheckResult`](FtSpellCheckResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.spellcheck/>](https://redis.io/commands/ft.spellcheck/)
    #[must_use]
    fn ft_spellcheck(
        self,
        index: impl Serialize,
        query: impl Serialize,
        options: FtSpellCheckOptions,
    ) -> PreparedCommand<'a, Self, FtSpellCheckResult> {
        prepare_command(
            self,
            cmd("FT.SPELLCHECK").arg(index).arg(query).arg(options),
        )
    }

    /// Dump the contents of a synonym group
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    ///
    /// # Return
    /// This command returns a list of synonym terms and their synonym group ids.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.syndump/>](https://redis.io/commands/ft.syndump/)
    /// * [`Synonym support`](https://redis.io/docs/stack/search/reference/synonyms/)
    #[must_use]
    fn ft_syndump<R: Response>(self, index: impl Serialize) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("FT.SYNDUMP").arg(index))
    }

    /// Update a synonym group
    ///
    /// Use this command to create or update a synonym group with additional terms.
    /// The command triggers a scan of all documents.    ///
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `synonym_group_id` - synonym group to return.
    /// * `skip_initial_scan` - does not scan and index, and only documents that are indexed after the update are affected.
    /// * `terms` - terms to add to the synonym group
    ///
    /// # Return
    /// This command returns a list of synonym terms and their synonym group ids.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.synupdate/>](https://redis.io/commands/ft.synupdate/)
    /// * [`Synonym support`](https://redis.io/docs/stack/search/reference/synonyms/)
    #[must_use]
    fn ft_synupdate(
        self,
        index: impl Serialize,
        synonym_group_id: impl Serialize,
        skip_initial_scan: bool,
        terms: impl Serialize,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("FT.SYNUPDATE")
                .arg(index)
                .arg(synonym_group_id)
                .arg_if(skip_initial_scan, "SKIPINITIALSCAN")
                .arg(terms),
        )
    }

    /// Return a distinct set of values indexed in a Tag field
    ///
    /// Use this command if your tag indexes things like cities, categories, and so on.
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `field_name` - name of a Tag file defined in the schema.
    ///
    /// # Return
    /// A coolection reply of all distinct tags in the tag index.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.tagvals/>](https://redis.io/commands/ft.tagvals/)
    #[must_use]
    fn ft_tagvals<R: Response>(
        self,
        index: impl Serialize,
        field_name: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("FT.TAGVALS").arg(index).arg(field_name))
    }

    /// Add a suggestion string to an auto-complete suggestion dictionary
    ///
    /// The auto-complete suggestion dictionary is disconnected from the index definitions
    /// and leaves creating and updating suggestions dictionaries to the user.
    ///
    /// # Arguments
    /// * `key` - suggestion dictionary key.
    /// * `string` - suggestion string to index.
    /// * `score` - floating point number of the suggestion string's weight.
    /// * `options` - See [`FtSugAddOptions`](FtSugAddOptions)
    ///
    /// # Return
    /// the current size of the suggestion dictionary.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.sugadd/>](https://redis.io/commands/ft.sugadd/)
    #[must_use]
    fn ft_sugadd(
        self,
        key: impl Serialize,
        string: impl Serialize,
        score: f64,
        options: FtSugAddOptions,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(
            self,
            cmd("FT.SUGADD")
                .arg(key)
                .arg(string)
                .arg(score)
                .arg(options),
        )
    }

    /// Delete a string from a suggestion index
    ///
    /// # Arguments
    /// * `key` - suggestion dictionary key.
    /// * `string` - suggestion string to delete
    ///
    /// # Return
    /// * `true` - if the string was found and deleted
    /// * `false` - otherwise
    /// # See Also
    /// [<https://redis.io/commands/ft.sugdel/>](https://redis.io/commands/ft.sugdel/)
    #[must_use]
    fn ft_sugdel(
        self,
        key: impl Serialize,
        string: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("FT.SUGDEL").arg(key).arg(string))
    }

    /// Get completion suggestions for a prefix
    ///
    /// # Arguments
    /// * `key` - suggestion dictionary key.
    /// * `prefix` - prefix to complete on.
    /// * `options` - See [`FtSugGetOptions`](FtSugGetOptions)
    ///
    /// # Return
    /// A collection of the top suggestions matching the prefix
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.sugget/>](https://redis.io/commands/ft.sugget/)
    #[must_use]
    fn ft_sugget(
        self,
        key: impl Serialize,
        prefix: impl Serialize,
        options: FtSugGetOptions,
    ) -> PreparedCommand<'a, Self, Vec<FtSuggestion>> {
        prepare_command(self, cmd("FT.SUGGET").arg(key).arg(prefix).arg(options)).custom_converter(
            Box::new(|resp_buffer, command, _client| {
                let mut deserializer = RespDeserializer::new(&resp_buffer);
                Box::pin(future::ready(FtSuggestion::deserialize(
                    &mut deserializer,
                    command,
                )))
            }),
        )
    }

    /// Get the size of an auto-complete suggestion dictionary
    ///
    /// # Arguments
    /// * `key` - suggestion dictionary key.
    ///
    /// # Return
    /// The the current size of the suggestion dictionary.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.suglen/>](https://redis.io/commands/ft.suglen/)
    #[must_use]
    fn ft_suglen(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("FT.SUGLEN").arg(key))
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtVectorType {
    Float64,
    Float32,
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtVectorDistanceMetric {
    L2,
    IP,
    Cosine,
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtFlatVectorFieldAttributes {
    pub r#type: FtVectorType,
    pub dim: usize,
    pub distance_metric: FtVectorDistanceMetric,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_cap: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_size: Option<usize>,
}

impl FtFlatVectorFieldAttributes {
    pub fn new(ty: FtVectorType, dim: usize, distance_metric: FtVectorDistanceMetric) -> Self {
        Self {
            r#type: ty,
            dim,
            distance_metric,
            initial_cap: None,
            block_size: None,
        }
    }

    pub fn initial_cap(self, initial_cap: usize) -> Self {
        Self {
            initial_cap: Some(initial_cap),
            ..self
        }
    }

    pub fn block_size(self, block_size: usize) -> Self {
        Self {
            block_size: Some(block_size),
            ..self
        }
    }

    pub fn num_attributes(&self) -> usize {
        let mut num = 6;

        if self.initial_cap.is_some() {
            num += 2
        }

        if self.block_size.is_some() {
            num += 2
        }

        num
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtHnswVectorFieldAttributes {
    pub r#type: FtVectorType,
    pub dim: usize,
    pub distance_metric: FtVectorDistanceMetric,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_cap: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ef_construction: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ef_runtime: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsilon: Option<f64>,
}

impl FtHnswVectorFieldAttributes {
    pub fn new(ty: FtVectorType, dim: usize, distance_metric: FtVectorDistanceMetric) -> Self {
        Self {
            r#type: ty,
            dim,
            distance_metric,
            initial_cap: None,
            m: None,
            ef_construction: None,
            ef_runtime: None,
            epsilon: None,
        }
    }

    pub fn initial_cap(self, initial_cap: usize) -> Self {
        Self {
            initial_cap: Some(initial_cap),
            ..self
        }
    }
    pub fn m(self, m: usize) -> Self {
        Self { m: Some(m), ..self }
    }
    pub fn ef_construction(self, ef_construction: usize) -> Self {
        Self {
            ef_construction: Some(ef_construction),
            ..self
        }
    }
    pub fn ef_runtime(self, ef_runtime: usize) -> Self {
        Self {
            ef_runtime: Some(ef_runtime),
            ..self
        }
    }
    pub fn epsilon(self, epsilon: f64) -> Self {
        Self {
            epsilon: Some(epsilon),
            ..self
        }
    }

    pub fn num_attributes(&self) -> usize {
        let mut num = 6;

        if self.initial_cap.is_some() {
            num += 2
        }

        if self.m.is_some() {
            num += 2
        }

        if self.ef_construction.is_some() {
            num += 2
        }

        if self.ef_runtime.is_some() {
            num += 2
        }

        if self.epsilon.is_some() {
            num += 2
        }

        num
    }
}

#[derive(Debug, Clone)]
pub enum FtVectorFieldAlgorithm {
    /// Brute force algorithm.
    Flat(FtFlatVectorFieldAttributes),

    /// Hierarchical Navigable Small World algorithm.
    HNSW(FtHnswVectorFieldAttributes),
}

impl Serialize for FtVectorFieldAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;

        match self {
            FtVectorFieldAlgorithm::Flat(attributes) => {
                seq.serialize_element("FLAT")?;
                seq.serialize_element(&attributes.num_attributes())?;
                seq.serialize_element(attributes)?;
            }
            FtVectorFieldAlgorithm::HNSW(attributes) => {
                seq.serialize_element("HNSW")?;
                seq.serialize_element(&attributes.num_attributes())?;
                seq.serialize_element(attributes)?;
            }
        }

        seq.end()
    }
}

/// Field type used to declare an index schema
/// for the [`ft_create`](SearchCommands::ft_create) command
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtFieldType {
    /// Allows full-text search queries against the value in this attribute.
    #[default]
    Text,
    /// Allows exact-match queries, such as categories or primary keys,
    /// against the value in this attribute.
    ///
    /// For more information,
    /// see [`Tag Fields`](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/tags/).
    Tag,
    /// Allows numeric range queries against the value in this attribute.
    ///
    /// See [`query syntax docs`](https://redis.io/docs/latest/develop/ai/search-and-query/query/)
    /// for details on how to use numeric ranges.
    Numeric,
    /// Allows geographic range queries against the value in this attribute.
    ///
    /// The value of the attribute must be a string containing a longitude (first) and latitude separated by a comma.
    Geo,
    /// Allows vector similarity queries against the value in this attribute.
    ///
    /// For more information, see [`Vector Fields`](https://redis.io/docs/latest/develop/ai/search-and-query/vectors/).
    Vector(#[serde(skip_deserializing)] Option<FtVectorFieldAlgorithm>),
}

/// Phonetic algorithm and language used for the [`FtFieldSchema::phonetic`](FtFieldSchema::phonetic) associated function
///
/// For more information, see [`Phonetic Matching`](https://redis.io/docs/stack/search/reference/phonetic_matching).
#[derive(Debug, Deserialize, Serialize)]
pub enum FtPhoneticMatcher {
    /// Double metaphone for English
    #[serde(rename = "dm:en")]
    DmEn,
    /// Double metaphone for French
    #[serde(rename = "dm:fr")]
    DmFr,
    /// Double metaphone for Portuguese
    #[serde(rename = "dm:pt")]
    DmPt,
    /// Double metaphone for Spanish
    #[serde(rename = "dm:es")]
    DmEs,
}

/// field schema for the [`ft_create`](SearchCommands::ft_create) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtFieldSchema<'a> {
    #[serde(rename = "")]
    identifier: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#as: Option<&'a str>,
    #[serde(rename = "")]
    field_type: FtFieldType,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    sortable: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    unf: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nostem: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    noindex: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    phonetic: Option<FtPhoneticMatcher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    separator: Option<char>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    casesensitive: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withsuffixtrie: bool,
}

impl<'a> FtFieldSchema<'a> {
    /// * For hashes, is a field name within the hash.
    /// * For JSON, the identifier is a JSON Path expression.
    #[must_use]
    pub fn identifier(identifier: &'a str) -> Self {
        Self {
            identifier,
            ..Default::default()
        }
    }

    /// Defines the attribute associated to the identifier.
    ///
    ///  For example, you can use this feature to alias a complex JSONPath
    ///  expression with more memorable (and easier to type) name.
    #[must_use]
    pub fn as_attribute(mut self, as_attribute: &'a str) -> Self {
        self.r#as = Some(as_attribute);
        self
    }

    /// The field type.
    ///
    /// Mandatory option to be used after `identifier` or `as_attribute`
    #[must_use]
    pub fn field_type(mut self, field_type: FtFieldType) -> Self {
        self.field_type = field_type;
        self
    }

    /// Numeric, tag (not supported with JSON) or text attributes can have the optional `SORTABLE` argument.
    ///
    /// As the user [`sorts the results by the value of this attribute`](https://redis.io/docs/stack/search/reference/sorting),
    /// the results will be available with very low latency.
    /// (this adds memory overhead so consider not to declare it on large text attributes).
    #[must_use]
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    /// By default, SORTABLE applies a normalization to the indexed value (characters set to lowercase, removal of diacritics).
    ///  When using un-normalized form (UNF), you can disable the normalization and keep the original form of the value.
    #[must_use]
    pub fn unf(mut self) -> Self {
        self.unf = true;
        self
    }

    /// Text attributes can have the `NOSTEM` argument which will disable stemming when indexing its values.
    /// This may be ideal for things like proper names.
    #[must_use]
    pub fn nostem(mut self) -> Self {
        self.nostem = true;
        self
    }

    /// Attributes can have the `NOINDEX` option, which means they will not be indexed.
    ///
    /// This is useful in conjunction with `SORTABLE`,
    /// to create attributes whose update using PARTIAL will not cause full reindexing of the document.
    /// If an attribute has NOINDEX and doesn't have SORTABLE, it will just be ignored by the index.
    #[must_use]
    pub fn noindex(mut self) -> Self {
        self.noindex = true;
        self
    }

    /// Declaring a text attribute as `PHONETIC` will perform phonetic matching on it in searches by default.
    ///
    /// The obligatory `matcher` argument specifies the phonetic algorithm and language used.
    #[must_use]
    pub fn phonetic(mut self, matcher: FtPhoneticMatcher) -> Self {
        self.phonetic = Some(matcher);
        self
    }

    /// for `TEXT` attributes, declares the importance of this attribute when calculating result accuracy.
    ///
    /// This is a multiplication factor, and defaults to 1 if not specified.
    #[must_use]
    pub fn weight(mut self, weight: f64) -> Self {
        self.weight = Some(weight);
        self
    }

    /// for `TAG` attributes, indicates how the text contained in the attribute is to be split into individual tags.
    /// The default is `,`. The value must be a single character.
    #[must_use]
    pub fn separator(mut self, sep: char) -> Self {
        self.separator = Some(sep);
        self
    }

    /// for `TAG` attributes, keeps the original letter cases of the tags.
    /// If not specified, the characters are converted to lowercase.
    #[must_use]
    pub fn case_sensitive(mut self) -> Self {
        self.casesensitive = true;
        self
    }

    /// for `TEXT` and `TAG` attributes, keeps a suffix [`trie`](https://en.wikipedia.org/wiki/Trie)
    ///  with all terms which match the suffix.
    ///
    /// It is used to optimize `contains` (foo) and `suffix` (*foo) queries.
    /// Otherwise, a brute-force search on the trie is performed.
    /// If suffix trie exists for some fields, these queries will be disabled for other fields.
    #[must_use]
    pub fn with_suffix_trie(mut self) -> Self {
        self.withsuffixtrie = true;
        self
    }
}

/// Redis Data type of an index defined in [`FtCreateOptions`](FtCreateOptions) struct
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtIndexDataType {
    /// [`hash`](https://redis.io/docs/data-types/hashes/) (default)
    Hash,
    /// [`json`](https://redis.io/docs/stack/json)
    Json,
}

/// Options for the [`ft_create`](SearchCommands::ft_create) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtCreateOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<FtIndexDataType>,
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    prefix: SmallVec<[&'a str; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<FtLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_field: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score_field: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload_field: Option<&'a str>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    maxtextfields: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nooffsets: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temporary: Option<u64>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nohl: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nofields: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nofreqs: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    skipinitialscan: bool,
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    stopwords: SmallVec<[&'a str; 10]>,
}

impl<'a> FtCreateOptions<'a> {
    /// currently supports HASH (default) and JSON.
    /// To index JSON, you must have the [`RedisJSON`](https://redis.io/docs/stack/json) module installed.
    #[must_use]
    pub fn on(mut self, data_type: FtIndexDataType) -> Self {
        self.on = Some(data_type);
        self
    }

    /// tells the index which keys it should index.
    ///
    /// Can be called multiple times to add several prefixes to index.
    /// Because the argument is optional, the default is * (all keys).
    #[must_use]
    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.prefix.push(prefix);
        self
    }

    /// filter expression with the full RediSearch aggregation expression language.
    ///
    /// It is possible to use `@__key` to access the key that was just added/changed.
    /// A field can be used to set field name by passing `FILTER @indexName=="myindexname"`.
    #[must_use]
    pub fn filter(mut self, filter: &'a str) -> Self {
        self.filter = Some(filter);
        self
    }

    /// if set, indicates the default language for documents in the index.
    ///
    /// Default to English.
    ///
    /// A stemmer is used for the supplied language during indexing.
    /// If an unsupported language is sent, the command returns an error.
    /// The supported languages are Arabic, Basque, Catalan, Danish, Dutch,
    /// English, Finnish, French, German, Greek, Hungarian, Indonesian, Irish,
    /// Italian, Lithuanian, Nepali, Norwegian, Portuguese, Romanian, Russian,
    /// Spanish, Swedish, Tamil, Turkish, and Chinese.
    ///
    /// When adding Chinese language documents, set `LANGUAGE` chinese for the indexer
    /// to properly tokenize the terms. If you use the default language,
    /// then search terms are extracted based on punctuation characters and whitespace.
    /// The Chinese language tokenizer makes use of a segmentation algorithm
    /// (via [`Friso`](https://github.com/lionsoul2014/friso)),
    /// which segments text and checks it against a predefined dictionary.
    /// See [`Stemming`](https://redis.io/docs/stack/search/reference/stemming) for more information.
    #[must_use]
    pub fn language(mut self, default_lang: FtLanguage) -> Self {
        self.language = Some(default_lang);
        self
    }

    /// document attribute set as the document language.
    #[must_use]
    pub fn language_field(mut self, lang_attribute: &'a str) -> Self {
        self.language_field = Some(lang_attribute);
        self
    }

    /// default score for documents in the index.
    ///
    /// Default score is 1.0.
    #[must_use]
    pub fn score(mut self, default_score: f64) -> Self {
        self.score = Some(default_score);
        self
    }

    /// document attribute that you use as the document rank based on the user ranking.
    ///
    /// Ranking must be between 0.0 and 1.0. If not set, the default score is 1.
    #[must_use]
    pub fn score_field(mut self, score_attribute: &'a str) -> Self {
        self.score_field = Some(score_attribute);
        self
    }

    /// document attribute that you use as a binary safe payload string to the document
    /// that can be evaluated at query time by a custom scoring function or retrieved to the client.
    #[must_use]
    pub fn payload_field(mut self, payload_attribute: &'a str) -> Self {
        self.payload_field = Some(payload_attribute);
        self
    }

    /// forces RediSearch to encode indexes as if there were more than 32 text attributes,
    /// which allows you to add additional attributes (beyond 32) using [`ft_alter`](SearchCommands::ft_alter).
    ///
    /// For efficiency, RediSearch encodes indexes differently if they are created with less than 32 text attributes.
    #[must_use]
    pub fn max_text_fields(mut self) -> Self {
        self.maxtextfields = true;
        self
    }

    /// does not store term offsets for documents.
    ///
    /// It saves memory, but does not allow exact searches or highlighting.
    /// It implies [`NOHL`](FtCreateOptions::nohl).
    #[must_use]
    pub fn no_offsets(mut self) -> Self {
        self.nooffsets = true;
        self
    }

    /// creates a lightweight temporary index that expires after a specified period of inactivity.
    ///
    /// * `expiration_sec` - index will expire after this duration in seconds.
    ///
    /// The internal idle timer is reset whenever the index is searched or added to.
    /// Because such indexes are lightweight,
    /// you can create thousands of such indexes without negative performance implications and, therefore,
    /// you should consider using [`SKIPINITIALSCAN`](FtCreateOptions::skip_initial_scan) to avoid costly scanning.
    #[must_use]
    pub fn temporary(mut self, expiration_sec: u64) -> Self {
        self.temporary = Some(expiration_sec);
        self
    }

    /// conserves storage space and memory by disabling highlighting support.
    ///
    /// If set, the corresponding byte offsets for term positions are not stored.
    /// `NOHL` is also implied by [`NOOFFSETS`](FtCreateOptions::no_offsets).
    #[must_use]
    pub fn nohl(mut self) -> Self {
        self.nohl = true;
        self
    }

    /// does not store attribute bits for each term.
    ///
    /// It saves memory, but it does not allow filtering by specific attributes.
    #[must_use]
    pub fn nofields(mut self) -> Self {
        self.nofields = true;
        self
    }

    /// avoids saving the term frequencies in the index.
    ///
    /// It saves memory, but does not allow sorting based on the frequencies of a given term within the document.
    #[must_use]
    pub fn nofreqs(mut self) -> Self {
        self.nofreqs = true;
        self
    }

    /// if set, does not scan and index.
    #[must_use]
    pub fn skip_initial_scan(mut self) -> Self {
        self.skipinitialscan = true;
        self
    }

    /// sets the index with a custom stopword list, to be ignored during indexing and search time.
    /// Can be called multiple times
    ///
    /// # Arguments
    /// * `stop_word` - a stopword argument.
    ///
    /// If not set, [`FT.CREATE`](SearchCommands::ft_create) takes the default list of stopwords.
    /// If `count` is set to 0, the index does not have stopwords.
    #[must_use]
    pub fn stop_word(mut self, stop_word: &'a str) -> Self {
        self.stopwords.push(stop_word);
        self
    }
}

/// Options for the [`ft_create`](SearchCommands::ft_aggregate) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtAggregateOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    verbatim: bool,
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    load: SmallVec<[FtAttribute<'a>; 10]>,
    #[serde(rename = "", skip_serializing_if = "SmallVec::is_empty")]
    expressions: SmallVec<[FtAggregateExpression<'a>; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<(u32, u32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    withcursor: Option<FtWithCursorOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    params: SmallVec<[(&'a str, &'a str); 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scorer: Option<FtScorerOptions<'a>>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    addscores: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    dialect: Option<u64>,
}

impl<'a> FtAggregateOptions<'a> {
    /// if set, does not try to use stemming for query expansion but searches the query terms verbatim.
    ///
    /// Attributes needed for aggregations should be stored as [`SORTABLE`](FtFieldSchema::sortable),
    /// where they are available to the aggregation pipeline with very low latency.
    /// `LOAD` hurts the performance of aggregate queries considerably because every processed record
    /// needs to execute the equivalent of [`HMGET`](crate::commands::HashCommands::hmget) against a Redis key,
    /// which when executed over millions of keys, amounts to high processing times.
    #[must_use]
    pub fn verbatim(mut self) -> Self {
        self.verbatim = true;
        self
    }

    /// loads document attributes from the source document.
    /// attribute: name of the attribute to load
    /// Can be called multiple times
    #[must_use]
    pub fn load(mut self, attribute: FtAttribute<'a>) -> Self {
        self.load.push(attribute);
        self
    }

    /// all attributes in a document are loaded.
    #[must_use]
    pub fn load_all(mut self) -> Self {
        self.load.push(FtAttribute::new("*"));
        self
    }

    /// groups the results in the pipeline based on one or more properties.
    ///
    /// Each group should have at least one reducer,
    /// a function that handles the group entries,
    /// either counting them,
    /// or performing multiple aggregate operations (see [`reduce`](FtAggregateOptions::reduce)).
    #[must_use]
    pub fn groupby(mut self, options: FtGroupBy<'a>) -> Self {
        self.expressions
            .push(FtAggregateExpression::GroupBy(options));
        self
    }

    /// Sort the pipeline up until the point of SORTBY, using a list of properties.
    ///
    /// `max` is used to optimized sorting, by sorting only for the n-largest elements.
    /// properties: collection of FtSortBy
    /// Although it is not connected to [`limit`](FtAggregateOptions::limit), you usually need just `SORTBY … MAX` for common queries.
    #[must_use]
    pub fn sortby(mut self, options: FtSortBy<'a>) -> Self {
        self.expressions
            .push(FtAggregateExpression::SortBy(options));
        self
    }

    /// applies a 1-to-1 transformation on one or more properties and either stores the result
    /// as a new property down the pipeline or replaces any property using this transformation.
    ///
    /// expr is an expression that can be used to perform arithmetic operations on numeric properties,
    /// or functions that can be applied on properties depending on their types (see below),
    /// or any combination thereof. For example, `APPLY "sqrt(@foo)/log(@bar) + 5" AS baz`
    /// evaluates this expression dynamically for each record in the pipeline and store the result as
    ///  a new property called baz, which can be referenced by further `APPLY`/`SORTBY`/`GROUPBY`/`REDUCE` operations down the pipeline.
    #[must_use]
    pub fn apply(mut self, expr: &'a str, as_name: &'a str) -> Self {
        self.expressions
            .push(FtAggregateExpression::Apply(FtApplyOptions::new(
                expr, as_name,
            )));
        self
    }

    /// Limits the number of results to return just num results starting at index offset (zero-based).
    ///
    /// It is much more efficient to use `SORTBY … MAX` if you are interested in just limiting the output of a sort operation.
    /// If a key expires during the query, an attempt to load the key's value will return a null array.
    ///
    /// However, limit can be used to limit results without sorting,
    /// or for paging the n-largest results as determined by `SORTBY MAX`.
    /// For example, getting results 50-100 of the top 100 results is most efficiently expressed as
    /// `SORTBY 1 @foo MAX 100 LIMIT 50 50`. Removing the `MAX` from `SORTBY` results in the pipeline
    /// sorting all the records and then paging over results 50-100.
    #[must_use]
    pub fn limit(mut self, offset: u32, num: u32) -> Self {
        self.limit = Some((offset, num));
        self
    }

    /// filters the results using predicate expressions relating to values in each result.
    /// They are applied post query and relate to the current state of the pipeline.
    #[must_use]
    pub fn filter<E, N>(mut self, expr: &'a str) -> Self {
        self.filter = Some(expr);
        self
    }

    /// Scan part of the results with a quicker alternative than [`limit`](FtAggregateOptions::limit).
    /// See [`Cursor API`](https://redis.io/docs/stack/search/reference/aggregations/#cursor-api) for more details.
    #[must_use]
    pub fn withcursor(mut self, options: FtWithCursorOptions) -> Self {
        self.withcursor = Some(options);
        self
    }

    /// if set, overrides the timeout parameter of the module.
    #[must_use]
    pub fn timeout(mut self, milliseconds: u64) -> Self {
        self.timeout = Some(milliseconds);
        self
    }

    /// defines one parameter. Each parameter has a name and a value.
    /// Can be called multiple times to add more parameters
    ///
    /// You can reference parameters in the query by a `$`,
    /// followed by the parameter name, for example, `$user`.
    ///
    /// Each such reference in the search query to a parameter name is substituted by the corresponding parameter value.
    /// For example, with parameter definition `params[("lon", 29.69465), ("lat", 34.95126)])`,
    /// the expression `@loc:[$lon $lat 10 km]` is evaluated to `@loc:[29.69465 34.95126 10 km]`.
    /// You cannot reference parameters in the query string where concrete values are not allowed,
    /// such as in field names, for example, @loc. To use `PARAMS`, set [`dialect`](FtAggregateOptions::dialect) to 2 or greater than 2.
    #[must_use]
    pub fn param(mut self, name: &'a str, value: &'a str) -> Self {
        self.params.push((name, value));
        self
    }

    /// uses a [built-in](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/scoring/)
    /// or a [user-provided](https://redis.io/docs/latest/develop/ai/search-and-query/administration/extensions/) scoring function.
    #[must_use]
    pub fn scorer(mut self, options: FtScorerOptions<'a>) -> Self {
        self.scorer = Some(options);
        self
    }

    /// The ADDSCORES option exposes the full-text score values to the aggregation pipeline.
    /// You can use @__score in a pipeline as shown in the following example:
    ///
    /// `FT.AGGREGATE idx 'hello' ADDSCORES SORTBY 2 @__score DESC`
    #[must_use]
    pub fn add_scores(mut self) -> Self {
        self.addscores = true;
        self
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(mut self, dialect_version: u64) -> Self {
        self.dialect = Some(dialect_version);
        self
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum FtAggregateExpression<'a> {
    GroupBy(FtGroupBy<'a>),
    SortBy(FtSortBy<'a>),
    Apply(FtApplyOptions<'a>),
}

/// Attribute for the [`LOAD`](FtAggregateOptions::load) aggregate option
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtAttribute<'a> {
    #[serde(rename = "")]
    identifier: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#as: Option<&'a str>,
}

impl<'a> FtAttribute<'a> {
    #[must_use]
    /// `identifier` is either an attribute name for hashes and JSON or a JSON Path expression for JSON.
    pub fn new(identifier: &'a str) -> Self {
        Self {
            identifier,
            ..Default::default()
        }
    }

    /// `property` is the optional name used in the result.
    ///
    /// If it is not provided, the identifier is used.
    /// This should be avoided.
    #[must_use]
    pub fn r#as(mut self, property: &'a str) -> Self {
        self.r#as = Some(property);
        self
    }
}

#[derive(Default, Serialize)]
pub struct FtGroupBy<'a> {
    #[serde(rename = "", serialize_with = "serialize_slice_with_len")]
    properties: SmallVec<[&'a str; 10]>,
    #[serde(rename = "", skip_serializing_if = "SmallVec::is_empty")]
    reducers: SmallVec<[FtReduceOptions<'a>; 10]>,
}

impl<'a> FtGroupBy<'a> {
    /// Add a property to the group by option
    pub fn property(mut self, property: &'a str) -> Self {
        self.properties.push(property);
        self
    }

    /// reduces the matching results in each group into a single record, using a reduction function.
    /// For example, COUNT counts the number of records in the group.
    /// The reducers can have their own property names using the AS {name} optional argument.
    /// If a name is not given, the resulting name will be the name of the reduce function and the group properties.
    /// For example, if a name is not given to COUNT_DISTINCT by property @foo,
    /// the resulting name will be count_distinct(@foo).
    ///
    /// See [Supported GROUPBY reducers](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/aggregations/#supported-groupby-reducers) for more details.
    pub fn reduce(mut self, reducer: FtReducer<'a>) -> Self {
        self.reducers.push(FtReduceOptions { reduce: reducer });
        self
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct FtReduceOptions<'a> {
    reduce: FtReducer<'a>,
}

impl<'a> From<FtReducer<'a>> for FtReduceOptions<'a> {
    fn from(reduce: FtReducer<'a>) -> Self {
        Self { reduce }
    }
}

/// Reducer for the [`groupby`](FtAggregateOptions::groupby) aggregate option
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtReducer<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count_distinct: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count_distinctish: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sum: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avg: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stddev: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantile: Option<(u32, &'a str, f64)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tolist: Option<(u32, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_value: Option<(u32, &'a str, Option<&'a str>, Option<SortOrder>)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    random_sample: Option<(u32, &'a str, u32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#as: Option<&'a str>,
}

impl<'a> FtReducer<'a> {
    #[must_use]
    /// Count the number of records in each group
    pub fn count() -> Self {
        Self {
            count: Some(0),
            ..Default::default()
        }
    }

    /// Count the number of distinct values for property.
    ///
    /// # Note
    /// The reducer creates a hash-set per group, and hashes each record.
    /// This can be memory heavy if the groups are big.
    pub fn count_distinct(property: &'a str) -> Self {
        Self {
            count_distinct: Some((1, property)),
            ..Default::default()
        }
    }

    /// Same as [`count_distinct`](FtReducer::count_distinct) - but provide an approximation instead of an exact count,
    /// at the expense of less memory and CPU in big groups.
    ///
    /// # Note
    /// The reducer uses [`HyperLogLog`](https://en.wikipedia.org/wiki/HyperLogLog) counters per group,
    /// at ~3% error rate, and 1024 Bytes of constant space allocation per group.
    /// This means it is ideal for few huge groups and not ideal for many small groups.
    /// In the former case, it can be an order of magnitude faster and consume much less memory
    /// than [`count_distinct`](FtReducer::count_distinct),
    /// but again, it does not fit every user case.
    pub fn count_distinctish(property: &'a str) -> Self {
        Self {
            count_distinctish: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the sum of all numeric values of a given property in a group.
    ///
    /// Non numeric values if the group are counted as 0.
    pub fn sum(property: &'a str) -> Self {
        Self {
            sum: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the minimal value of a property, whether it is a string, number or NULL.
    pub fn min(property: &'a str) -> Self {
        Self {
            min: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the maximal value of a property, whether it is a string, number or NULL.
    pub fn max(property: &'a str) -> Self {
        Self {
            max: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the average value of a numeric property.
    ///
    /// This is equivalent to reducing by sum and count,
    /// and later on applying the ratio of them as an APPLY step.
    pub fn avg(property: &'a str) -> Self {
        Self {
            avg: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the [`standard deviation`](https://en.wikipedia.org/wiki/Standard_deviation)
    /// of a numeric property in the group.
    pub fn stddev(property: &'a str) -> Self {
        Self {
            stddev: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the value of a numeric property at a given quantile of the results.
    ///
    /// Quantile is expressed as a number between 0 and 1.
    /// For example, the median can be expressed as the quantile at 0.5, e.g. REDUCE QUANTILE 2 @foo 0.5 AS median .
    /// If multiple quantiles are required, just repeat the QUANTILE reducer for each quantile.
    /// e.g. REDUCE QUANTILE 2 @foo 0.5 AS median REDUCE QUANTILE 2 @foo 0.99 AS p99
    pub fn quantile(property: &'a str, quantile: f64) -> Self {
        Self {
            quantile: Some((2, property, quantile)),
            ..Default::default()
        }
    }

    /// Merge all `distinct` values of a given property into a single array.
    pub fn tolist(property: &'a str) -> Self {
        Self {
            tolist: Some((1, property)),
            ..Default::default()
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    ///
    /// If no BY is specified, we return the first value we encounter in the group.
    /// If you with to get the top or bottom value in the group sorted by the same value,
    /// you are better off using the MIN/MAX reducers,
    /// but the same effect will be achieved by doing REDUCE FIRST_VALUE 4 @foo BY @foo DESC.
    pub fn first_value(property: &'a str) -> Self {
        Self {
            first_value: Some((1, property, None, None)),
            ..Default::default()
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    pub fn first_value_by(property: &'a str, by_property: &'a str) -> Self {
        Self {
            first_value: Some((2, property, Some(by_property), None)),
            ..Default::default()
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    pub fn first_value_by_order(property: &'a str, by_property: &'a str, order: SortOrder) -> Self {
        Self {
            first_value: Some((3, property, Some(by_property), Some(order))),
            ..Default::default()
        }
    }

    /// Perform a reservoir sampling of the group elements with a given size,
    ///  and return an array of the sampled items with an even distribution.
    pub fn random_sample(property: &'a str, sample_size: u32) -> Self {
        Self {
            random_sample: Some((2, property, sample_size)),
            ..Default::default()
        }
    }

    /// The reducers can have their own property names using the AS {name} optional argument.
    ///
    /// If a name is not given, the resulting name will be
    /// the name of the reduce function and the group properties.
    /// For example, if a name is not given to COUNT_DISTINCT by property @foo,
    /// the resulting name will be count_distinct(@foo).
    pub fn as_name(mut self, name: &'a str) -> Self {
        self.r#as = Some(name);
        self
    }
}

/// option for the [`sortby`](FtAggregateOptions::sortby) aggregate option
#[derive(Serialize)]
pub struct FtSortByProperty<'a>(&'a str, SortOrder);

impl<'a> FtSortByProperty<'a> {
    pub fn new(property: &'a str) -> Self {
        Self(property, SortOrder::Asc)
    }

    pub fn asc(mut self) -> Self {
        self.1 = SortOrder::Asc;
        self
    }

    pub fn desc(mut self) -> Self {
        self.1 = SortOrder::Desc;
        self
    }
}

/// option for the [`sortby`](FtAggregateOptions::sortby) aggregate option
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSortBy<'a> {
    #[serde(
        rename = "",
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    properties: SmallVec<[&'a str; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withcount: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withoutcount: bool,
}

impl<'a> FtSortBy<'a> {
    pub fn property(mut self, property: FtSortByProperty<'a>) -> Self {
        self.properties.push(property.0);
        match property.1 {
            SortOrder::Asc => self.properties.push("ASC"),
            SortOrder::Desc => self.properties.push("DESC"),
        }
        self
    }

    /// MAX is used to optimized sorting, by sorting only for the n-largest elements.
    /// Although it is not connected to LIMIT, you usually need just SORTBY … MAX for common queries.
    pub fn max(mut self, num: u32) -> Self {
        self.max = Some(num);
        self
    }

    pub fn with_count(mut self) -> Self {
        self.withcount = true;
        self
    }

    pub fn without_count(mut self) -> Self {
        self.withoutcount = true;
        self
    }
}

/// Options for the [`FT.AGGREGATE`](FtAggregateOptions::ft_aggregate) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct FtApplyOptions<'a> {
    #[serde(rename = "")]
    expression: &'a str,
    r#as: &'a str,
}

impl<'a> FtApplyOptions<'a> {
    #[must_use]
    pub fn new(expression: &'a str, as_name: &'a str) -> Self {
        Self {
            expression,
            r#as: as_name,
        }
    }
}

/// options for the [`withcursor`](FtAggregateOptions::withcursor) aggregate option
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtWithCursorOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maxidle: Option<u64>,
}

impl FtWithCursorOptions {
    /// Control how many rows are read per each cursor fetch.
    pub fn count(mut self, read_size: u32) -> FtWithCursorOptions {
        self.count = Some(read_size);
        self
    }

    /// Because cursors are stateful resources which occupy memory on the server, they have a limited lifetime.
    ///
    /// In order to safeguard against orphaned/stale cursors, cursors have an idle timeout value.
    /// If no activity occurs on the cursor before the idle timeout, the cursor is deleted.
    /// The idle timer resets to 0 whenever the cursor is read from using `CURSOR READ`.
    ///
    /// The default idle timeout is 300000 milliseconds (or 300 seconds).
    /// You can modify the idle timeout using the MAXIDLE keyword when creating the cursor.
    /// Note that the value cannot exceed the default 300s.
    pub fn maxidle(mut self, idle_time_ms: u64) -> FtWithCursorOptions {
        self.maxidle = Some(idle_time_ms);
        self
    }
}

/// options for the [`scorer`](FtAggregateOptions::scorer) aggregate option
#[derive(Serialize)]
pub enum FtScorerOptions<'a> {
    #[serde(rename = "TFIDF")]
    TfIdf,
    #[serde(rename = "TFIDF.DOCNORM")]
    TfIdfDocNorm,
    #[serde(rename = "BM25STD")]
    Bm25Std,
    #[serde(rename = "BM25STD.NORM")]
    Bm25StdNorm,
    #[serde(rename = "BM25STD.TANH")]
    Bm25StdTanh {
        #[serde(rename = "BM25STD_TANH_FACTOR")]
        factor: f64,
    },
    #[serde(rename = "DISMAX")]
    DisMax,
    #[serde(rename = "DISMAX")]
    DOCSCORE,
    #[serde(rename = "HAMMING")]
    Hamming,
    #[serde(rename = "")]
    Custom(&'a str),
}

/* */
/// Result for the [`ft_aggregate`](SearchCommands::ft_aggregate) command
#[derive(Debug)]
pub struct FtAggregateResult {
    pub attributes: Vec<String>,
    pub format: String,
    pub results: Vec<FtSearchResultRow>,
    pub total_results: usize,
    pub warning: Vec<String>,
    pub cursor_id: Option<u64>,
}

impl<'de> Deserialize<'de> for FtAggregateResult {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FtAggregateResultVisitor;

        impl<'de> Visitor<'de> for FtAggregateResultVisitor {
            type Value = FtAggregateResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("FtAggregateResult")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let Some(result) = seq.next_element::<FtSearchResult>()? else {
                    return Err(de::Error::invalid_length(0, &"2 elements in sequence"));
                };

                let Some(cursor) = seq.next_element::<u64>()? else {
                    return Err(de::Error::invalid_length(0, &"2 elements in sequence"));
                };

                Ok(FtAggregateResult {
                    attributes: result.attributes,
                    format: result.format,
                    results: result.results,
                    total_results: result.total_results,
                    warning: result.warning,
                    cursor_id: Some(cursor),
                })
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let result = FtSearchResult::deserialize(MapAccessDeserializer::new(map))?;
                Ok(FtAggregateResult {
                    attributes: result.attributes,
                    format: result.format,
                    results: result.results,
                    total_results: result.total_results,
                    warning: result.warning,
                    cursor_id: None,
                })
            }
        }

        deserializer.deserialize_any(FtAggregateResultVisitor)
    }
}

/// Result for the [`ft_search`](SearchCommands::ft_search) and [`ft_aggregate`](SearchCommands::ft_aggregate) commands
#[derive(Debug, Deserialize)]
pub struct FtSearchResult {
    pub attributes: Vec<String>,
    pub format: String,
    pub results: Vec<FtSearchResultRow>,
    pub total_results: usize,
    pub warning: Vec<String>,
}

/// A row in a [`FtSearchResult`](FtSearchResult)
#[derive(Debug, Default, Deserialize)]
pub struct FtSearchResultRow {
    /// Document id. Will be empty for [`ft_aggregate`](SearchCommands::ft_aggregate)
    #[serde(default)]
    pub id: String,
    /// relative internal score of each document. only if [`withscores`](FtSearchOptions::withscores) is set
    #[serde(default)]
    pub score: f64,
    /// document payload. only if [`withpayloads`](FtSearchOptions::withpayloads) is set
    #[serde(default)]
    pub payload: String,
    /// value of the sorting key. only if [`withsortkeys`](FtSearchOptions::withsortkeys) is set
    #[serde(default)]
    pub sortkey: String,
    /// collection of attribute/value pairs.
    pub values: Vec<(String, String)>,
    /// collection of attribute/value pairs.
    #[serde(default)]
    pub extra_attributes: Vec<(String, String)>,
}

/// Result for the [`ft_info`](SearchCommands::ft_info) command
#[derive(Debug, Deserialize)]
pub struct FtInfoResult {
    /// Name of the index
    pub index_name: String,
    /// index [`creation`](SearchCommands::ft_create) options without paramater
    pub index_options: Vec<String>,
    /// index [`creation`](SearchCommands::ft_create) options with a paramater
    pub index_definition: FtIndexDefinition,
    /// index attributes
    pub attributes: Vec<FtIndexAttribute>,
    /// Number of documents.
    pub num_docs: usize,
    /// Max document id
    pub max_doc_id: u64,
    /// Number of distinct terms.
    pub num_terms: usize,
    pub num_records: usize,
    pub inverted_sz_mb: f64,
    pub vector_index_sz_mb: f64,
    pub total_inverted_index_blocks: usize,
    pub offset_vectors_sz_mb: f64,
    pub doc_table_size_mb: f64,
    pub sortable_values_size_mb: f64,
    pub key_table_size_mb: f64,
    pub tag_overhead_sz_mb: f64,
    pub text_overhead_sz_mb: f64,
    pub total_index_memory_sz_mb: f64,
    pub geoshapes_sz_mb: f64,
    pub records_per_doc_avg: f64,
    pub bytes_per_record_avg: f64,
    pub offsets_per_term_avg: f64,
    pub offset_bits_per_record_avg: f64,
    /// number of failures due to operations not compatible with index schema.
    pub hash_indexing_failures: usize,
    pub total_indexing_time: f64,
    /// whether of not the index is being scanned in the background.
    pub indexing: bool,
    /// progress of background indexing (1 if complete).
    pub percent_indexed: f64,
    /// The number of times the index has been used.
    pub number_of_uses: usize,
    /// The index deletion flag. A value of true indicates index deletion is in progress.
    pub cleaning: bool,
    #[serde(default)]
    pub gc_stats: Option<FtGcStats>,
    #[serde(default)]
    pub cursor_stats: Option<FtCursorStats>,
    /// if a custom stopword list is used.
    #[serde(default)]
    pub stopwords_list: Vec<String>,
    pub dialect_stats: HashMap<String, usize>,
}

/// Index attribute info
#[derive(Debug, Default)]
pub struct FtIndexAttribute {
    /// field identifier
    pub identifier: String,
    /// attribute associated to the identifier
    pub attribute: String,
    /// field type
    pub field_type: FtFieldType,
    /// weight defined on the field. Defaults to 1
    pub weight: f64,
    /// true if the field is sortable
    pub sortable: bool,
    /// true if the field has un-normalized form
    pub unf: bool,
    /// true if stemming is disable for this fied
    pub no_stem: bool,
    /// true if this field is not indexed
    pub no_index: bool,
    /// Phoentic matcher
    pub phonetic: Option<FtPhoneticMatcher>,
    /// tag separator
    pub separator: Option<char>,
    /// case sensitivity for tags
    pub case_sensitive: bool,
    /// suffixe trie
    pub with_suffixe_trie: bool,
}

impl<'de> Deserialize<'de> for FtIndexAttribute {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FlagsSeed<'a> {
            attribute: &'a mut FtIndexAttribute,
        }

        impl<'a> FlagsSeed<'a> {
            pub fn new(attribute: &'a mut FtIndexAttribute) -> Self {
                Self { attribute }
            }
        }

        impl<'de, 'a> de::DeserializeSeed<'de> for FlagsSeed<'a> {
            type Value = ();

            fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_seq(self)
            }
        }

        impl<'de, 'a> Visitor<'de> for FlagsSeed<'a> {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of flags")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                while let Some(flag) = seq.next_element::<&str>()? {
                    match flag {
                        "SORTABLE" => self.attribute.sortable = true,
                        "UNF" => self.attribute.unf = true,
                        "NOSTEM" => self.attribute.no_stem = true,
                        "NOINDEX" => self.attribute.no_index = true,
                        "CASESENSITIVE" => self.attribute.case_sensitive = true,
                        "WITHSUFFIXTRIE" => self.attribute.with_suffixe_trie = true,
                        _ => (),
                    }
                }
                Ok(())
            }
        }

        struct FtIndexAttributeVisitor;

        impl<'de> Visitor<'de> for FtIndexAttributeVisitor {
            type Value = FtIndexAttribute;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("FtIndexAttribute")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut attribute = FtIndexAttribute::default();

                while let Some(field_name) = map.next_key::<&str>()? {
                    match field_name {
                        "identifier" => {
                            attribute.identifier = map.next_value::<String>()?;
                        }
                        "attribute" => {
                            attribute.attribute = map.next_value::<String>()?;
                        }
                        "type" => {
                            attribute.field_type = map.next_value::<FtFieldType>()?;
                        }
                        "WEIGHT" => {
                            attribute.weight = map.next_value::<f64>()?;
                        }
                        "flags" => {
                            map.next_value_seed(FlagsSeed::new(&mut attribute))?;
                        }
                        "SEPARATOR" => attribute.separator = Some(map.next_value::<char>()?),
                        "PHONETIC" => {
                            attribute.phonetic = Some(map.next_value::<FtPhoneticMatcher>()?)
                        }
                        _ => (),
                    }
                }
                Ok(attribute)
            }
        }

        deserializer.deserialize_map(FtIndexAttributeVisitor)
    }
}

/// Garbage collector stats for the [`ft_info`](SearchCommands::ft_info) command
#[derive(Debug, Deserialize)]
pub struct FtGcStats {
    pub bytes_collected: usize,
    pub total_ms_run: usize,
    pub total_cycles: usize,
    pub average_cycle_time_ms: f64,
    pub last_run_time_ms: usize,
    pub gc_numeric_trees_missed: usize,
    pub gc_blocks_denied: usize,
}

/// Cursor stats for the [`ft_info`](SearchCommands::ft_info) command
#[derive(Debug, Deserialize)]
pub struct FtCursorStats {
    pub global_idle: usize,
    pub global_total: usize,
    pub index_capacity: usize,
    pub index_total: usize,
}

/// Index definitin for the [`ft_info`](SearchCommands::ft_info) command
#[derive(Debug, Deserialize)]
pub struct FtIndexDefinition {
    pub key_type: FtIndexDataType,
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub filter: String,
    #[serde(default)]
    pub default_language: String,
    #[serde(default)]
    pub language_field: String,
    pub default_score: f64,
    #[serde(default)]
    pub score_field: String,
    #[serde(default)]
    pub payload_field: String,
    #[serde(default)]
    pub indexes_all: String,
}

/// Options for the [`ft_search`](SearchCommands::ft_search) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSearchOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nocontent: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    verbatim: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nostopwords: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withscores: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withpayloads: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withsortkeys: bool,
    #[serde(rename = "", skip_serializing_if = "SmallVec::is_empty")]
    filter: SmallVec<[FtFilterOptions<'a>; 10]>,
    #[serde(rename = "", skip_serializing_if = "SmallVec::is_empty")]
    geofilter: SmallVec<[FtGeoFilterOptions<'a>; 10]>,
    #[serde(
        rename = "",
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    inkeys: SmallVec<[&'a str; 10]>,
    #[serde(
        rename = "",
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    infields: SmallVec<[&'a str; 10]>,
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    r#return: SmallVec<[FtAttribute<'a>; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    summarize: Option<FtSearchSummarizeOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    highlight: Option<FtSearchHighlightOptions<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slop: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u64>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    inorder: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<FtLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expander: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scorer: Option<FtScorerOptions<'a>>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    explainscore: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<&'a [u8]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sortby: Option<(&'a str, SortOrder)>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withcount: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withoutcount: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<(u32, u32)>,
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    params: SmallVec<[(&'a str, &'a str); 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dialect: Option<u64>,
}

impl<'a> FtSearchOptions<'a> {
    /// returns the document ids and not the content.
    /// This is useful if RediSearch is only an index on an external document collection.
    #[must_use]
    pub fn nocontent(mut self) -> Self {
        self.nocontent = true;
        self
    }

    /// does not try to use stemming for query expansion but searches the query terms verbatim.
    #[must_use]
    pub fn verbatim(mut self) -> Self {
        self.verbatim = true;
        self
    }

    /// also returns the relative internal score of each document.
    ///
    /// This can be used to merge results from multiple instances.
    #[must_use]
    pub fn withscores(mut self) -> Self {
        self.withscores = true;
        self
    }

    /// retrieves optional document payloads.
    ///
    /// See [`ft_create`](SearchCommands::ft_create)
    /// The payloads follow the document id and, if [`withscores`](FtSearchOptions::withscores) is set, the scores.
    #[must_use]
    pub fn withpayloads(mut self) -> Self {
        self.withpayloads = true;
        self
    }

    /// returns the value of the sorting key, right after the id and score and/or payload, if requested.
    ///
    /// This is usually not needed, and exists for distributed search coordination purposes.
    /// This option is relevant only if used in conjunction with [`sortby`](FtSearchOptions::sortby).
    #[must_use]
    pub fn withsortkeys(mut self) -> Self {
        self.withsortkeys = true;
        self
    }

    /// limits results to those having numeric values ranging between min and max,
    /// if numeric_field is defined as a numeric field in [`ft_create`](SearchCommands::ft_create).
    ///
    /// `min` and `max` follow [`zrange`](crate::commands::SortedSetCommands::zrange) syntax, and can be `-inf`, `+inf`,
    /// and use `(` for exclusive ranges. Multiple numeric filters for different attributes are supported in one query.
    #[must_use]
    pub fn filter(mut self, numeric_field: &'a str, min: &'a str, max: &'a str) -> Self {
        self.filter.push(FtFilterOptions {
            filter: (numeric_field, min, max),
        });
        self
    }

    /// filter the results to a given `radius` from `lon` and `lat`.
    ///
    /// `radius` is given as a number and units.
    /// See [`geosearch`](crate::commands::GeoCommands::geosearch) for more details.
    #[must_use]
    pub fn geo_filter(
        mut self,
        geo_field: &'a str,
        longitude: f64,
        latitude: f64,
        radius: f64,
        unit: GeoUnit,
    ) -> Self {
        self.geofilter.push(FtGeoFilterOptions {
            geofilter: (geo_field, longitude, latitude, radius, unit),
        });
        self
    }

    /// limits the result to a given set of keys specified in the list.
    ///
    /// Non-existent keys are ignored, unless all the keys are non-existent.
    ///
    /// Call multiple times to add multiple keys
    #[must_use]
    pub fn inkey<A>(mut self, key: &'a str) -> Self {
        self.inkeys.push(key);
        self
    }

    /// filters the results to those appearing only in specific attributes of the document, like `title` or `URL`.
    ///
    /// Call multiple times to add multiple fields
    #[must_use]
    pub fn infields<A>(mut self, field: &'a str) -> Self {
        self.infields.push(field);
        self
    }

    /// limits the attributes returned from the document.
    ///
    /// attributes: collection of FtSearchReturnAttribute
    /// If attributes is empty, it acts like [`nocontent`](FtSearchOptions::nocontent).
    ///
    /// Call multiple times to add multiple attributes
    #[must_use]
    pub fn _return(mut self, attribute: FtAttribute<'a>) -> Self {
        self.r#return.push(attribute);
        self
    }

    /// returns only the sections of the attribute that contain the matched text.
    ///
    /// See [`Highlighting`](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/highlight/) for more information.
    #[must_use]
    pub fn summarize(mut self, options: FtSearchSummarizeOptions<'a>) -> Self {
        self.summarize = Some(options);
        self
    }

    /// formats occurrences of matched text.
    ///
    /// See [`Highlighting`](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/highlight/) for more information.
    #[must_use]
    pub fn highlight(mut self, options: FtSearchHighlightOptions<'a>) -> Self {
        self.highlight = Some(options);
        self
    }

    /// allows a maximum of N intervening number of unmatched offsets between phrase terms.
    ///
    /// In other words, the slop for exact phrases is 0.
    #[must_use]
    pub fn slop(mut self, slop: u32) -> Self {
        self.slop = Some(slop);
        self
    }

    /// overrides the timeout parameter of the module.
    #[must_use]
    pub fn timeout(mut self, milliseconds: u64) -> Self {
        self.timeout = Some(milliseconds);
        self
    }

    /// puts the query terms in the same order in the document as in the query,
    /// regardless of the offsets between them.
    ///
    /// Typically used in conjunction with [`slop`](FtSearchOptions::slop).
    #[must_use]
    pub fn inorder(mut self) -> Self {
        self.inorder = true;
        self
    }

    /// use a stemmer for the supplied language during search for query expansion.
    ///
    /// If querying documents in Chinese, set to chinese to properly tokenize the query terms.
    /// Defaults to English.
    /// If an unsupported language is sent, the command returns an error.
    /// See FT.CREATE for the list of languages.
    #[must_use]
    pub fn language(mut self, language: FtLanguage) -> Self {
        self.language = Some(language);
        self
    }

    /// uses a custom query `expander` instead of the stemmer.
    ///
    /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn expander(mut self, expander: &'a str) -> Self {
        self.expander = Some(expander);
        self
    }

    /// uses a [built-in](https://redis.io/docs/latest/develop/ai/search-and-query/advanced-concepts/scoring/)
    /// or a [user-provided](https://redis.io/docs/latest/develop/ai/search-and-query/administration/extensions/) scoring function.
    #[must_use]
    pub fn scorer(mut self, options: FtScorerOptions<'a>) -> Self {
        self.scorer = Some(options);
        self
    }

    /// returns a textual description of how the scores were calculated.
    ///
    /// Using this options requires the [`withscores`](FtSearchOptions::withscores) option.
    #[must_use]
    pub fn explainscore(mut self) -> Self {
        self.explainscore = true;
        self
    }

    /// adds an arbitrary, binary safe `payload` that is exposed to custom scoring functions.
    ///
    /// See [`Extensions`]((https://redis.io/docs/latest/develop/ai/search-and-query/administration/extensions/).
    #[must_use]
    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }

    /// orders the results by the value of this attribute.
    ///
    /// This applies to both text and numeric attributes.
    /// Attributes needed for `sortby` should be declared as [`SORTABLE`](FtFieldSchema::sortable) in the index,
    /// in order to be available with very low latency. Note that this adds memory overhead.
    #[must_use]
    pub fn sortby(mut self, attribute: &'a str, order: SortOrder, with_count: bool) -> Self {
        self.sortby = Some((attribute, order));
        if with_count {
            self.withcount = true;
            self.withoutcount = false;
        } else {
            self.withcount = false;
            self.withoutcount = true;
        }
        self
    }

    /// limits the results to the offset and number of results given.
    ///
    /// Note that the offset is zero-indexed. The default is `0 10`, which returns 10 items starting from the first result.
    /// You can use `LIMIT 0 0` to count the number of documents in the result set without actually returning them.
    #[must_use]
    pub fn limit(mut self, offset: u32, num: u32) -> Self {
        self.limit = Some((offset, num));
        self
    }

    /// defines one parameter. Each parameter has a name and a value.
    /// Can be called multiple times to add more parameters
    ///
    /// You can reference parameters in the query by a `$`,
    /// followed by the parameter name, for example, `$user`.
    ///
    /// Each such reference in the search query to a parameter name is substituted by the corresponding parameter value.
    /// For example, with parameter definition `params[("lon", 29.69465), ("lat", 34.95126)])`,
    /// the expression `@loc:[$lon $lat 10 km]` is evaluated to `@loc:[29.69465 34.95126 10 km]`.
    /// You cannot reference parameters in the query string where concrete values are not allowed,
    /// such as in field names, for example, @loc. To use `PARAMS`, set [`dialect`](FtSearchOptions::dialect) to 2 or greater than 2.
    #[must_use]
    pub fn param(mut self, name: &'a str, value: &'a str) -> Self {
        self.params.push((name, value));
        self
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(mut self, dialect_version: u64) -> Self {
        self.dialect = Some(dialect_version);
        self
    }
}

#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct FtFilterOptions<'a> {
    filter: (&'a str, &'a str, &'a str),
}

#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct FtGeoFilterOptions<'a> {
    geofilter: (&'a str, f64, f64, f64, GeoUnit),
}

/// sub-options for the [`search`](SearchCommands::ft_search) option [`summarize`](FtSearchOptions::summarize)
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSearchSummarizeOptions<'a> {
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    fields: SmallVec<[&'a str; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frags: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    len: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    separator: Option<&'a str>,
}

impl<'a> FtSearchSummarizeOptions<'a> {
    /// If present, must be the first argument.
    /// Each field present is summarized.
    /// If no `FIELDS` directive is passed, then all fields returned are summarized.
    ///
    /// Call multiple times to add multiple fields
    #[must_use]
    pub fn field(mut self, field: &'a str) -> Self {
        self.fields.push(field);
        self
    }

    /// How many fragments should be returned. If not specified, a default of 3 is used.
    #[must_use]
    pub fn frags(mut self, num_frags: u32) -> Self {
        self.frags = Some(num_frags);
        self
    }

    /// The number of context words each fragment should contain.
    ///
    /// Context words surround the found term.
    /// A higher value will return a larger block of text.
    /// If not specified, the default value is 20.
    #[must_use]
    pub fn len(mut self, frag_len: u32) -> Self {
        self.len = Some(frag_len);
        self
    }

    /// The string used to divide between individual summary snippets.
    ///
    /// The default is `...` which is common among search engines;
    /// but you may override this with any other string if you desire to programmatically divide them later on.
    /// You may use a newline sequence, as newlines are stripped from the result body anyway
    /// (thus, it will not be conflated with an embedded newline in the text)
    #[must_use]
    pub fn separator(mut self, separator: &'a str) -> Self {
        self.separator = Some(separator);
        self
    }
}

/// sub-options for the [`search`](SearchCommands::ft_search) option [`summarize`](FtSearchOptions::highlight)
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSearchHighlightOptions<'a> {
    #[serde(
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    fields: SmallVec<[&'a str; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<(&'a str, &'a str)>,
}

impl<'a> FtSearchHighlightOptions<'a> {
    /// If present, must be the first argument.
    /// Each field present is highlighted.
    /// If no `FIELDS` directive is passed, then all fields returned are highlighted.
    ///
    /// Call multiple times to add multiple fields
    #[must_use]
    pub fn fields(mut self, field: &'a str) -> Self {
        self.fields.push(field);
        self
    }

    /// * `open_tag` - prepended to each term match
    /// * `close_tag` - appended to each term match
    ///   If no `TAGS` are specified, a built-in tag value is appended and prepended.
    #[must_use]
    pub fn tags(mut self, open_tag: &'a str, close_tag: &'a str) -> Self {
        self.tags = Some((open_tag, close_tag));
        self
    }
}

/// Redis search supported languages
/// See. [`Supported Languages`](https://redis.io/docs/stack/search/reference/stemming/#supported-languages)
#[derive(Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FtLanguage {
    Arabic,
    Armenian,
    Basque,
    Catalan,
    Chinese,
    Danish,
    Dutch,
    #[default]
    English,
    Finnish,
    French,
    German,
    Greek,
    Hungarian,
    Indonesian,
    Irish,
    Italian,
    Lithuanian,
    Nepali,
    Norwegian,
    Portuguese,
    Romanian,
    Russian,
    Serbian,
    Spanish,
    Swedish,
    Tamil,
    Turkish,
    Yiddish,
}

/// Options for the [`ft_spellcheck`](SearchCommands::ft_spellcheck) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSpellCheckOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    distance: Option<u64>,
    #[serde(
        rename = "",
        skip_serializing_if = "SmallVec::is_empty",
        serialize_with = "serialize_slice_with_len"
    )]
    terms: SmallVec<[FtSpellCheckTermsOption<'a>; 10]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dialect: Option<u64>,
}

impl<'a> FtSpellCheckOptions<'a> {
    /// maximum Levenshtein distance for spelling suggestions (default: 1, max: 4).
    #[must_use]
    pub fn distance(mut self, distance: u64) -> Self {
        self.distance = Some(distance);
        self
    }

    /// specifies an inclusion (`FtTermType::Include`) or exclusion (`FtTermType::Exclude`) of a custom dictionary named `dictionary`
    ///
    /// Refer to [`ft_dictadd`](SearchCommands::ft_dictadd), [`ft_dictdel`](SearchCommands::ft_dictdel)
    /// and [`ft_dictdump`](SearchCommands::ft_dictdump) about managing custom dictionaries.
    #[must_use]
    pub fn terms(mut self, term_type: FtTermType, dictionary: &'a str) -> Self {
        self.terms.push(FtSpellCheckTermsOption {
            terms: (term_type, dictionary),
        });
        self
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(mut self, dialect_version: u64) -> Self {
        self.dialect = Some(dialect_version);
        self
    }
}

/// Term type for the option [`terms`](FtSpellCheckOptions::terms)
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct FtSpellCheckTermsOption<'a> {
    terms: (FtTermType, &'a str),
}

/// Term type for the option [`terms`](FtSpellCheckOptions::terms)
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtTermType {
    Include,
    Exclude,
}

/// Result for the [`ft_spellcheck`](SearchCommands::ft_spellcheck) command.
#[derive(Debug, Deserialize)]
pub struct FtSpellCheckResult {
    /// a collection where each element represents a misspelled term from the query + suggestions for this term
    ///
    /// The misspelled terms are ordered by their order of appearance in the query.
    #[serde(rename = "results", deserialize_with = "deserialize_misspelled_terms")]
    pub misspelled_terms: Vec<FtMisspelledTerm>,
}

/// Misspelled term + suggestions for the [`ft_spellcheck`](SearchCommands::ft_spellcheck) command.
#[derive(Debug, Deserialize)]
pub struct FtMisspelledTerm {
    /// Misspelled term
    pub misspelled_term: String,
    /// Suggestion as a tuple composed of
    /// * the suggestion
    /// * the score of the suggestion
    pub suggestions: Vec<(String, f64)>,
}

fn deserialize_misspelled_terms<'de, D>(deserializer: D) -> Result<Vec<FtMisspelledTerm>, D::Error>
where
    D: Deserializer<'de>,
{
    struct SuggestionSeed;

    impl<'de> DeserializeSeed<'de> for SuggestionSeed {
        type Value = (String, f64);

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct Visitor;

            impl<'de> de::Visitor<'de> for Visitor {
                type Value = (String, f64);

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a (String, f64)")
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: de::MapAccess<'de>,
                {
                    let Some(suggestion) = map.next_entry()? else {
                        return Err(de::Error::custom("Cannot parse misspelled terms"));
                    };

                    Ok(suggestion)
                }
            }

            deserializer.deserialize_map(Visitor)
        }
    }

    struct SuggestionsSeed;

    impl<'de> DeserializeSeed<'de> for SuggestionsSeed {
        type Value = Vec<(String, f64)>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct Visitor;

            impl<'de> de::Visitor<'de> for Visitor {
                type Value = Vec<(String, f64)>;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a Vec<(String, f64)>")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: de::SeqAccess<'de>,
                {
                    let mut suggestions = Vec::with_capacity(seq.size_hint().unwrap_or_default());
                    while let Some(suggestion) = seq.next_element_seed(SuggestionSeed)? {
                        suggestions.push(suggestion);
                    }

                    Ok(suggestions)
                }
            }

            deserializer.deserialize_seq(Visitor)
        }
    }

    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<FtMisspelledTerm>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an array of FtMisspelledTerm")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut result = Vec::with_capacity(map.size_hint().unwrap_or_default());
            while let Some(misspelled_term) = map.next_key()? {
                let suggestions = map.next_value_seed(SuggestionsSeed)?;
                result.push(FtMisspelledTerm {
                    misspelled_term,
                    suggestions,
                });
            }

            Ok(result)
        }
    }

    deserializer.deserialize_map(Visitor)
}

/// Options for the [`ft_sugadd`](SearchCommands::ft_sugadd) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSugAddOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    incr: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<&'a [u8]>,
}

impl<'a> FtSugAddOptions<'a> {
    /// increments the existing entry of the suggestion by the given score, instead of replacing the score.
    ///
    /// This is useful for updating the dictionary based on user queries in real time.
    #[must_use]
    pub fn incr(mut self) -> Self {
        self.incr = true;
        self
    }

    /// saves an extra payload with the suggestion, that can be fetched by adding the
    /// [`WITHPAYLOADS`](FtSugGetOptions::withpayload) argument to [`FT.SUGGET`](SearchCommands::ft_sugget).
    #[must_use]
    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }
}

/// Options for the [`ft_sugget`](SearchCommands::ft_sugget) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct FtSugGetOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    fuzzy: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withscores: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withpayloads: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<u32>,
}

impl FtSugGetOptions {
    /// performs a fuzzy prefix search, including prefixes at Levenshtein distance of 1 from the prefix sent.
    #[must_use]
    pub fn fuzzy(mut self) -> Self {
        self.fuzzy = true;
        self
    }

    /// returns the score of each suggestion.
    ///
    /// This can be used to merge results from multiple instances.
    #[must_use]
    pub fn withscores(mut self) -> Self {
        self.withscores = true;
        self
    }

    /// returns optional payloads saved along with the suggestions.
    ///
    /// If no payload is present for an entry, it returns a null reply.
    #[must_use]
    pub fn withpayloads(mut self) -> Self {
        self.withpayloads = true;
        self
    }

    /// limits the results to a maximum of `num` (default: 5).
    #[must_use]
    pub fn max(mut self, num: u32) -> Self {
        self.max = Some(num);
        self
    }
}

/// Sugestion for the [`ft_sugget`](SearchCommands::ft_sugget) command.
#[derive(Deserialize)]
pub struct FtSuggestion {
    pub suggestion: String,
    pub score: f64,
    pub payload: String,
}

impl FtSuggestion {
    pub fn deserialize<'de, D>(
        deserializer: D,
        command: Command,
    ) -> std::result::Result<Vec<FtSuggestion>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FtSuggestionVecVisitor {
            command: Command,
        }

        impl<'de> Visitor<'de> for FtSuggestionVecVisitor {
            type Value = Vec<FtSuggestion>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Vec<FtSuggestion>")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut with_scores = false;
                let mut with_payloads = false;
                for i in 0..self.command.num_args() {
                    if let Some(arg) = self.command.get_arg(i) {
                        if arg == b"WITHSCORES" {
                            with_scores = true;
                        }

                        if arg == b"WITHPAYLOADS" {
                            with_payloads = true;
                        }
                    }
                }

                let mut suggestions = if let Some(size) = seq.size_hint() {
                    Vec::with_capacity(size)
                } else {
                    Vec::new()
                };

                while let Some(suggestion) = seq.next_element()? {
                    let score = if with_scores {
                        let Some(score) = seq.next_element()? else {
                            return Err(de::Error::custom("Cannot parse FtSuggestion"));
                        };
                        score
                    } else {
                        0.
                    };

                    let payload = if with_payloads {
                        let Some(payload) = seq.next_element()? else {
                            return Err(de::Error::custom("Cannot parse FtSuggestion"));
                        };
                        payload
                    } else {
                        String::from("")
                    };

                    suggestions.push(FtSuggestion {
                        suggestion,
                        score,
                        payload,
                    });
                }

                Ok(suggestions)
            }
        }

        deserializer.deserialize_seq(FtSuggestionVecVisitor { command })
    }
}
