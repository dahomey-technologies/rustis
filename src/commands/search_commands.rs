use crate::{
    client::{prepare_command, PreparedCommand},
    commands::{GeoUnit, SortOrder},
    resp::{
        cmd, deserialize_vec_of_pairs, CollectionResponse, Command, CommandArgs,
        KeyValueCollectionResponse, MultipleArgsCollection, PrimitiveResponse, RespDeserializer,
        SingleArg, SingleArgCollection, ToArgs, Value, VecOfPairsSeed,
    },
};
use serde::{
    de::{self, value::SeqAccessDeserializer, DeserializeOwned, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use std::{collections::HashMap, fmt, future};

/// A group of Redis commands related to [`RedisSearch`](https://redis.io/docs/stack/search/)
///
/// # See Also
/// * [RedisSearch Commands](https://redis.io/commands/?group=search)
/// * [Auto-Suggest Commands](https://redis.io/commands/?group=suggestion)
pub trait SearchCommands<'a> {
    /// Run a search query on an index,
    /// and perform aggregate transformations on the results,
    /// extracting statistics etc from them
    ///
    /// # Arguments
    /// * `index` - index against which the query is executed.
    /// * `query`- is base filtering query that retrieves the documents.\
    ///  It follows the exact same syntax as the search query,\
    ///  including filters, unions, not, optional, and so on.
    /// * `options` - See [`FtAggregateOptions`](FtAggregateOptions)
    ///
    /// # Returns
    /// An instance of [`FtAggregateResult`](FtAggregateResult)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.aggregate/>](https://redis.io/commands/ft.aggregate/)
    /// * [`RedisSeach Aggregations`](https://redis.io/docs/stack/search/reference/aggregations/)
    #[must_use]
    fn ft_aggregate<I, Q>(
        self,
        index: I,
        query: Q,
        options: FtAggregateOptions,
    ) -> PreparedCommand<'a, Self, FtAggregateResult>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
    {
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
    fn ft_aliasadd<A, I>(self, alias: A, index: I) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        A: SingleArg,
        I: SingleArg,
    {
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
    fn ft_aliasdel<A>(self, alias: A) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        A: SingleArg,
    {
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
    fn ft_aliasupdate<A, I>(self, alias: A, index: I) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        A: SingleArg,
        I: SingleArg,
    {
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
    fn ft_alter<I>(
        self,
        index: I,
        skip_initial_scan: bool,
        attribute: FtFieldSchema,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        I: SingleArg,
    {
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
    fn ft_config_get<O, N, V, R>(self, option: O) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        O: SingleArg,
        N: PrimitiveResponse,
        V: PrimitiveResponse,
        R: KeyValueCollectionResponse<N, V>,
    {
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
    fn ft_config_set<O, V>(self, option: O, value: V) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        O: SingleArg,
        V: SingleArg,
    {
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
    fn ft_create<I, S>(
        self,
        index: I,
        options: FtCreateOptions,
        schema: S,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        I: SingleArg,
        S: MultipleArgsCollection<FtFieldSchema>,
    {
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
    fn ft_cursor_del<I>(self, index: I, cursor_id: u64) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        I: SingleArg,
    {
        prepare_command(self, cmd("FT.CURSOR").arg("DEL").arg(index).arg(cursor_id))
    }

    /// Read next results from an existing cursor
    ///
    /// # Arguments
    /// * `index` - index name.
    /// * `cursor_id` - id of the cursor.
    /// * `read_size` - number of results to read. This parameter overrides
    /// [`count`](FtWithCursorOptions::count) specified in [`ft_aggregate`](SearchCommands::ft_aggregate).
    ///
    /// # Returns
    /// an instance of [`FtAggregateResult`](FtAggregateResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.cursor-read/>](https://redis.io/commands/ft.cursor-read/)
    #[must_use]
    fn ft_cursor_read<I>(
        self,
        index: I,
        cursor_id: u64,
    ) -> PreparedCommand<'a, Self, FtAggregateResult>
    where
        Self: Sized,
        I: SingleArg,
    {
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
    fn ft_dictadd<D, T, TT>(self, dict: D, terms: TT) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        T: SingleArg,
        TT: SingleArgCollection<T>,
    {
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
    fn ft_dictdel<D, T, TT>(self, dict: D, terms: TT) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
        D: SingleArg,
        T: SingleArg,
        TT: SingleArgCollection<T>,
    {
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
    fn ft_dictdump<D, T, TT>(self, dict: D) -> PreparedCommand<'a, Self, TT>
    where
        Self: Sized,
        D: SingleArg,
        T: PrimitiveResponse + DeserializeOwned,
        TT: CollectionResponse<T>,
    {
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
    /// Adding the `dd` option deletes the hashes as well.
    /// * When using `ft_dropindex` with the parameter `dd`, if an index creation is still running
    /// ([`ft_create`](SearchCommands::ft_create) is running asynchronously),
    /// only the document hashes that have already been indexed are deleted.
    /// The document hashes left to be indexed remain in the database.
    /// You can use [`ft_info`](SearchCommands::ft_info) to check the completion of the indexing.
    ///
    /// # Return
    /// the number of new terms that were added.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.dropindex/>](https://redis.io/commands/ft.dropindex/)
    #[must_use]
    fn ft_dropindex<I>(self, index: I, dd: bool) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
        I: SingleArg,
    {
        prepare_command(self, cmd("FT.DROPINDEX").arg(index).arg_if(dd, "DD"))
    }

    /// Return the execution plan for a complex query
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `query` - query string, as if sent to [`ft_search`](SearchCommands::ft_search).
    /// * `dialect_version` - dialect version under which to execute the query. \
    ///  If not specified, the query executes under the default dialect version set during module initial loading\
    ///  or via [`ft_config_set`](SearchCommands::ft_config_set) command.
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
    fn ft_explain<I, Q, R>(
        self,
        index: I,
        query: Q,
        dialect_version: Option<u64>,
    ) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
        R: PrimitiveResponse,
    {
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
    ///  If not specified, the query executes under the default dialect version set during module initial loading\
    ///  or via [`ft_config_set`](SearchCommands::ft_config_set) command.
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
    fn ft_explaincli<I, Q, R, RR>(
        self,
        index: I,
        query: Q,
        dialect_version: Option<u64>,
    ) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
        R: PrimitiveResponse + DeserializeOwned,
        RR: CollectionResponse<R>,
    {
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
    fn ft_info(self, index: impl SingleArg) -> PreparedCommand<'a, Self, FtInfoResult>
    where
        Self: Sized,
    {
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
    fn ft_list<R, RR>(self) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
        R: PrimitiveResponse + DeserializeOwned,
        RR: CollectionResponse<R>,
    {
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
    /// An instance of [`FtProfileSearchResult`](FtProfileSearchResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.profile/>](https://redis.io/commands/ft.profile/)
    #[must_use]
    fn ft_profile_search<I, Q, QQ>(
        self,
        index: I,
        limited: bool,
        query: QQ,
    ) -> PreparedCommand<'a, Self, FtProfileSearchResult>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
        QQ: SingleArgCollection<Q>,
    {
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
    /// An instance of [`FtProfileAggregateResult`](FtProfileAggregateResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.profile/>](https://redis.io/commands/ft.profile/)
    #[must_use]
    fn ft_profile_aggregate<I, Q, QQ>(
        self,
        index: I,
        limited: bool,
        query: QQ,
    ) -> PreparedCommand<'a, Self, FtProfileAggregateResult>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
        QQ: SingleArgCollection<Q>,
    {
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
    fn ft_search<I, Q>(
        self,
        index: I,
        query: Q,
        options: FtSearchOptions,
    ) -> PreparedCommand<'a, Self, FtSearchResult>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
    {
        prepare_command(self, cmd("FT.SEARCH").arg(index).arg(query).arg(options))
    }

    /// Perform spelling correction on a query, returning suggestions for misspelled terms
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](SearchCommands::ft_create).
    /// * `query` - search query. See [`Spellchecking`](https://redis.io/docs/stack/search/reference/spellcheck) for more details.
    /// * `options` - See [`FtSpellCheckOptions`](FtSpellCheckOptions)
    ///
    /// # Return
    /// An instance of [`FtSpellCheckResult`](FtSpellCheckResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.spellcheck/>](https://redis.io/commands/ft.spellcheck/)
    #[must_use]
    fn ft_spellcheck<I, Q>(
        self,
        index: I,
        query: Q,
        options: FtSpellCheckOptions,
    ) -> PreparedCommand<'a, Self, FtSpellCheckResult>
    where
        Self: Sized,
        I: SingleArg,
        Q: SingleArg,
    {
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
    fn ft_syndump<I, R>(self, index: I) -> PreparedCommand<'a, Self, R>
    where
        Self: Sized,
        I: SingleArg,
        R: KeyValueCollectionResponse<String, Vec<String>>,
    {
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
    fn ft_synupdate<T: SingleArg>(
        self,
        index: impl SingleArg,
        synonym_group_id: impl SingleArg,
        skip_initial_scan: bool,
        terms: impl SingleArgCollection<T>,
    ) -> PreparedCommand<'a, Self, ()>
    where
        Self: Sized,
    {
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
    fn ft_tagvals<R: PrimitiveResponse + DeserializeOwned, RR: CollectionResponse<R>>(
        self,
        index: impl SingleArg,
        field_name: impl SingleArg,
    ) -> PreparedCommand<'a, Self, RR>
    where
        Self: Sized,
    {
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
        key: impl SingleArg,
        string: impl SingleArg,
        score: f64,
        options: FtSugAddOptions,
    ) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
    {
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
        key: impl SingleArg,
        string: impl SingleArg,
    ) -> PreparedCommand<'a, Self, bool>
    where
        Self: Sized,
    {
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
        key: impl SingleArg,
        prefix: impl SingleArg,
        options: FtSugGetOptions,
    ) -> PreparedCommand<'a, Self, Vec<FtSuggestion>>
    where
        Self: Sized,
    {
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
    fn ft_suglen(self, key: impl SingleArg) -> PreparedCommand<'a, Self, usize>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FT.SUGLEN").arg(key))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FtVectorType {
    Float64,
    Float32,
}

impl ToArgs for FtVectorType {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            FtVectorType::Float32 => "FLOAT32",
            FtVectorType::Float64 => "FLOAT64",
        });
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FtVectorDistanceMetric {
    L2,
    IP,
    Cosine,
}

impl ToArgs for FtVectorDistanceMetric {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            FtVectorDistanceMetric::L2 => "L2",
            FtVectorDistanceMetric::IP => "IP",
            FtVectorDistanceMetric::Cosine => "COSINE",
        });
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FtFlatVectorFieldAttributes {
    pub ty: FtVectorType,
    pub dim: usize,
    pub distance_metric: FtVectorDistanceMetric,
    pub initial_cap: Option<usize>,
    pub block_size: Option<usize>,
}

impl FtFlatVectorFieldAttributes {
    pub fn new(ty: FtVectorType, dim: usize, distance_metric: FtVectorDistanceMetric) -> Self {
        Self {
            ty,
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
}

impl ToArgs for FtFlatVectorFieldAttributes {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg("TYPE")
            .arg(self.ty)
            .arg("DIM")
            .arg(self.dim)
            .arg("DISTANCE_METRIC")
            .arg(self.distance_metric);

        if let Some(initial_cap) = self.initial_cap {
            args.arg("INITIAL_CAP").arg(initial_cap);
        }

        if let Some(block_size) = self.block_size {
            args.arg("BLOCK_SIZE").arg(block_size);
        }
    }

    fn num_args(&self) -> usize {
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

#[derive(Debug, Copy, Clone)]
pub struct FtHnswVectorFieldAttributes {
    pub ty: FtVectorType,
    pub dim: usize,
    pub distance_metric: FtVectorDistanceMetric,
    pub initial_cap: Option<usize>,
    pub m: Option<usize>,
    pub ef_construction: Option<usize>,
    pub ef_runtime: Option<usize>,
    pub epsilon: Option<f64>,
}

impl FtHnswVectorFieldAttributes {
    pub fn new(ty: FtVectorType, dim: usize, distance_metric: FtVectorDistanceMetric) -> Self {
        Self {
            ty,
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
}

impl ToArgs for FtHnswVectorFieldAttributes {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg("TYPE")
            .arg(self.ty)
            .arg("DIM")
            .arg(self.dim)
            .arg("DISTANCE_METRIC")
            .arg(self.distance_metric);

        if let Some(initial_cap) = self.initial_cap {
            args.arg("INITIAL_CAP").arg(initial_cap);
        }

        if let Some(m) = self.m {
            args.arg("M").arg(m);
        }

        if let Some(ef_construction) = self.ef_construction {
            args.arg("EF_CONSTRUCTION").arg(ef_construction);
        }

        if let Some(ef_runtime) = self.ef_runtime {
            args.arg("EF_RUNTIME").arg(ef_runtime);
        }

        if let Some(epsilon) = self.epsilon {
            args.arg("EPSILON").arg(epsilon);
        }
    }

    fn num_args(&self) -> usize {
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

impl ToArgs for FtVectorFieldAlgorithm {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            FtVectorFieldAlgorithm::Flat(attr) => {
                args.arg("FLAT");
                args.arg(attr.num_args());
                attr.write_args(args);
            }
            FtVectorFieldAlgorithm::HNSW(attr) => {
                args.arg("HNSW");
                args.arg(attr.num_args());
                attr.write_args(args);
            }
        }
    }

    fn num_args(&self) -> usize {
        let num_attrs = match self {
            FtVectorFieldAlgorithm::Flat(attr) => attr.num_args(),
            FtVectorFieldAlgorithm::HNSW(attr) => attr.num_args(),
        };

        2 + num_attrs
    }
}

/// Field type used to declare an index schema
/// for the [`ft_create`](SearchCommands::ft_create) command
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtFieldType {
    /// Allows full-text search queries against the value in this attribute.
    #[default]
    Text,
    /// Allows exact-match queries, such as categories or primary keys,
    /// against the value in this attribute.
    ///
    /// For more information,
    /// see [`Tag Fields`](https://redis.io/docs/stack/search/reference/tags).
    Tag,
    /// Allows numeric range queries against the value in this attribute.
    ///
    /// See [`query syntax docs`](https://redis.io/docs/stack/search/reference/query_syntax)
    /// for details on how to use numeric ranges.
    Numeric,
    /// Allows geographic range queries against the value in this attribute.
    ///
    /// The value of the attribute must be a string containing a longitude (first) and latitude separated by a comma.
    Geo,
    /// Allows vector similarity queries against the value in this attribute.
    ///
    /// For more information, see [`Vector Fields`](https://redis.io/docs/stack/search/reference/vectors).
    Vector(#[serde(skip)] Option<FtVectorFieldAlgorithm>),
}

impl ToArgs for FtFieldType {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            FtFieldType::Text => {
                args.arg("TEXT");
            }
            FtFieldType::Tag => {
                args.arg("TAG");
            }
            FtFieldType::Numeric => {
                args.arg("NUMERIC");
            }
            FtFieldType::Geo => {
                args.arg("GEO");
            }
            FtFieldType::Vector(ty) => {
                args.arg("VECTOR");
                ty.write_args(args)
            }
        }
    }
}

/// Phonetic algorithm and language used for the [`FtFieldSchema::phonetic`](FtFieldSchema::phonetic) associated function
///
/// For more information, see [`Phonetic Matching`](https://redis.io/docs/stack/search/reference/phonetic_matching).
#[derive(Debug, Deserialize)]
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

impl ToArgs for FtPhoneticMatcher {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            FtPhoneticMatcher::DmEn => "dm:en",
            FtPhoneticMatcher::DmFr => "dm:fr",
            FtPhoneticMatcher::DmPt => "dm:pt",
            FtPhoneticMatcher::DmEs => "dm:es",
        });
    }
}

/// field schema for the [`ft_create`](SearchCommands::ft_create) command
#[derive(Default)]
pub struct FtFieldSchema {
    command_args: CommandArgs,
}

impl FtFieldSchema {
    /// * For hashes, is a field name within the hash.
    /// * For JSON, the identifier is a JSON Path expression.
    #[must_use]
    pub fn identifier<N: SingleArg>(identifier: N) -> Self {
        Self {
            command_args: CommandArgs::default().arg(identifier).build(),
        }
    }

    /// Defines the attribute associated to the identifier.
    ///
    ///  For example, you can use this feature to alias a complex JSONPath
    ///  expression with more memorable (and easier to type) name.
    #[must_use]
    pub fn as_attribute<A: SingleArg>(mut self, as_attribute: A) -> Self {
        Self {
            command_args: self.command_args.arg("AS").arg(as_attribute).build(),
        }
    }

    /// The field type.
    ///
    /// Mandatory option to be used after `identifier` or `as_attribute`
    #[must_use]
    pub fn field_type(mut self, field_type: FtFieldType) -> Self {
        Self {
            command_args: self.command_args.arg(field_type).build(),
        }
    }

    /// Numeric, tag (not supported with JSON) or text attributes can have the optional `SORTABLE` argument.
    ///
    /// As the user [`sorts the results by the value of this attribute`](https://redis.io/docs/stack/search/reference/sorting),
    /// the results will be available with very low latency.
    /// (this adds memory overhead so consider not to declare it on large text attributes).
    #[must_use]
    pub fn sortable(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("SORTABLE").build(),
        }
    }

    /// By default, SORTABLE applies a normalization to the indexed value (characters set to lowercase, removal of diacritics).
    ///  When using un-normalized form (UNF), you can disable the normalization and keep the original form of the value.
    #[must_use]
    pub fn unf(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("UNF").build(),
        }
    }

    /// Text attributes can have the `NOSTEM` argument which will disable stemming when indexing its values.
    /// This may be ideal for things like proper names.
    #[must_use]
    pub fn nostem(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOSTEM").build(),
        }
    }

    /// Attributes can have the `NOINDEX` option, which means they will not be indexed.
    ///
    /// This is useful in conjunction with `SORTABLE`,
    /// to create attributes whose update using PARTIAL will not cause full reindexing of the document.
    /// If an attribute has NOINDEX and doesn't have SORTABLE, it will just be ignored by the index.
    #[must_use]
    pub fn noindex(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOINDEX").build(),
        }
    }

    /// Declaring a text attribute as `PHONETIC` will perform phonetic matching on it in searches by default.
    ///
    /// The obligatory `matcher` argument specifies the phonetic algorithm and language used.
    #[must_use]
    pub fn phonetic(mut self, matcher: FtPhoneticMatcher) -> Self {
        Self {
            command_args: self.command_args.arg("PHONETIC").arg(matcher).build(),
        }
    }

    /// for `TEXT` attributes, declares the importance of this attribute when calculating result accuracy.
    ///
    /// This is a multiplication factor, and defaults to 1 if not specified.
    #[must_use]
    pub fn weight(mut self, weight: f64) -> Self {
        Self {
            command_args: self.command_args.arg("WEIGHT").arg(weight).build(),
        }
    }

    /// for `TAG` attributes, indicates how the text contained in the attribute is to be split into individual tags.
    /// The default is `,`. The value must be a single character.
    #[must_use]
    pub fn separator(mut self, sep: char) -> Self {
        Self {
            command_args: self.command_args.arg("SEPARATOR").arg(sep).build(),
        }
    }

    /// for `TAG` attributes, keeps the original letter cases of the tags.
    /// If not specified, the characters are converted to lowercase.
    #[must_use]
    pub fn case_sensitive(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("CASESENSITIVE").build(),
        }
    }

    /// for `TEXT` and `TAG` attributes, keeps a suffix [`trie`](https://en.wikipedia.org/wiki/Trie)
    ///  with all terms which match the suffix.
    ///
    /// It is used to optimize `contains` (foo) and `suffix` (*foo) queries.
    /// Otherwise, a brute-force search on the trie is performed.
    /// If suffix trie exists for some fields, these queries will be disabled for other fields.
    #[must_use]
    pub fn with_suffix_trie(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSUFFIXTRIE").build(),
        }
    }
}

impl ToArgs for FtFieldSchema {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Redis Data type of an index defined in [`FtCreateOptions`](FtCreateOptions) struct
#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FtIndexDataType {
    /// [`hash`](https://redis.io/docs/data-types/hashes/) (default)
    Hash,
    /// [`json`](https://redis.io/docs/stack/json)
    Json,
}

impl ToArgs for FtIndexDataType {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            FtIndexDataType::Hash => "HASH",
            FtIndexDataType::Json => "JSON",
        });
    }
}

/// Options for the [`ft_create`](SearchCommands::ft_create) command
#[derive(Default)]
pub struct FtCreateOptions {
    command_args: CommandArgs,
}

impl FtCreateOptions {
    /// currently supports HASH (default) and JSON.
    /// To index JSON, you must have the [`RedisJSON`](https://redis.io/docs/stack/json) module installed.
    #[must_use]
    pub fn on(mut self, data_type: FtIndexDataType) -> Self {
        Self {
            command_args: self.command_args.arg("ON").arg(data_type).build(),
        }
    }

    /// tells the index which keys it should index.
    ///
    /// You can add several prefixes to index.
    /// Because the argument is optional, the default is * (all keys).
    #[must_use]
    pub fn prefix<P: SingleArg, PP: SingleArgCollection<P>>(mut self, prefixes: PP) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("PREFIX")
                .arg(prefixes.num_args())
                .arg(prefixes)
                .build(),
        }
    }

    /// filter expression with the full RediSearch aggregation expression language.
    ///
    /// It is possible to use `@__key` to access the key that was just added/changed.
    /// A field can be used to set field name by passing `FILTER @indexName=="myindexname"`.
    #[must_use]
    pub fn filter<F: SingleArg>(mut self, filter: F) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER").arg(filter).build(),
        }
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
        Self {
            command_args: self.command_args.arg("LANGUAGE").arg(default_lang).build(),
        }
    }

    /// document attribute set as the document language.
    #[must_use]
    pub fn language_field<L: SingleArg>(mut self, default_lang: L) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LANGUAGE_FIELD")
                .arg(default_lang)
                .build(),
        }
    }

    /// default score for documents in the index.
    ///
    /// Default score is 1.0.
    #[must_use]
    pub fn score(mut self, default_score: f64) -> Self {
        Self {
            command_args: self.command_args.arg("SCORE").arg(default_score).build(),
        }
    }

    /// document attribute that you use as the document rank based on the user ranking.
    ///
    /// Ranking must be between 0.0 and 1.0. If not set, the default score is 1.
    #[must_use]
    pub fn score_field<S: SingleArg>(mut self, score_attribute: S) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("SCORE_FIELD")
                .arg(score_attribute)
                .build(),
        }
    }

    /// document attribute that you use as a binary safe payload string to the document
    /// that can be evaluated at query time by a custom scoring function or retrieved to the client.
    #[must_use]
    pub fn payload_field<P: SingleArg>(mut self, payload_attribute: P) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("PAYLOAD_FIELD")
                .arg(payload_attribute)
                .build(),
        }
    }

    /// forces RediSearch to encode indexes as if there were more than 32 text attributes,
    /// which allows you to add additional attributes (beyond 32) using [`ft_alter`](SearchCommands::ft_alter).
    ///
    /// For efficiency, RediSearch encodes indexes differently if they are created with less than 32 text attributes.
    #[must_use]
    pub fn max_text_fields(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("MAXTEXTFIELDS").build(),
        }
    }

    /// does not store term offsets for documents.
    ///
    /// It saves memory, but does not allow exact searches or highlighting.
    /// It implies [`NOHL`](FtCreateOptions::nohl).
    #[must_use]
    pub fn no_offsets(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOOFFSETS").build(),
        }
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
        Self {
            command_args: self
                .command_args
                .arg("TEMPORARY")
                .arg(expiration_sec)
                .build(),
        }
    }

    /// conserves storage space and memory by disabling highlighting support.
    ///
    /// If set, the corresponding byte offsets for term positions are not stored.
    /// `NOHL` is also implied by [`NOOFFSETS`](FtCreateOptions::no_offsets).
    #[must_use]
    pub fn nohl(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOHL").build(),
        }
    }

    /// does not store attribute bits for each term.
    ///
    /// It saves memory, but it does not allow filtering by specific attributes.
    #[must_use]
    pub fn nofields(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOFIELDS").build(),
        }
    }

    /// avoids saving the term frequencies in the index.
    ///
    /// It saves memory, but does not allow sorting based on the frequencies of a given term within the document.
    #[must_use]
    pub fn nofreqs(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOFREQS").build(),
        }
    }

    /// if set, does not scan and index.
    #[must_use]
    pub fn skip_initial_scan(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("SKIPINITIALSCAN").build(),
        }
    }

    /// sets the index with a custom stopword list, to be ignored during indexing and search time.
    ///
    /// # Arguments
    /// * `stop_words` - a list of stopword arguments.
    ///
    /// If not set, [`FT.CREATE`](SearchCommands::ft_create) takes the default list of stopwords.
    /// If `count` is set to 0, the index does not have stopwords.
    #[must_use]
    pub fn stop_words<W, WW>(mut self, stop_words: WW) -> Self
    where
        W: SingleArg,
        WW: SingleArgCollection<W>,
    {
        Self {
            command_args: self
                .command_args
                .arg("STOPWORDS")
                .arg(stop_words.num_args())
                .arg(stop_words)
                .build(),
        }
    }
}

impl ToArgs for FtCreateOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`ft_create`](SearchCommands::ft_aggregate) command
#[derive(Default)]
pub struct FtAggregateOptions {
    command_args: CommandArgs,
}

impl FtAggregateOptions {
    /// if set, does not try to use stemming for query expansion but searches the query terms verbatim.
    ///
    /// Attributes needed for aggregations should be stored as [`SORTABLE`](FtFieldSchema::sortable),
    /// where they are available to the aggregation pipeline with very low latency.
    /// `LOAD` hurts the performance of aggregate queries considerably because every processed record
    /// needs to execute the equivalent of [`HMGET`](crate::commands::HashCommands::hmget) against a Redis key,
    /// which when executed over millions of keys, amounts to high processing times.
    #[must_use]
    pub fn verbatim(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("VERBATIM").build(),
        }
    }

    /// loads document attributes from the source document.
    #[must_use]
    pub fn load<A: MultipleArgsCollection<FtLoadAttribute>>(mut self, attributes: A) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LOAD")
                .arg(attributes.num_args())
                .arg(attributes)
                .build(),
        }
    }

    /// all attributes in a document are loaded.
    #[must_use]
    pub fn load_all(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("LOAD").arg("*").build(),
        }
    }

    /// groups the results in the pipeline based on one or more properties.
    ///
    /// Each group should have at least one reducer,
    /// a function that handles the group entries,
    /// either counting them,
    /// or performing multiple aggregate operations (see [`FtReducer`](FtReducer)).
    #[must_use]
    pub fn groupby<P, PP, R>(mut self, properties: PP, reducers: R) -> Self
    where
        P: SingleArg,
        PP: SingleArgCollection<P>,
        R: MultipleArgsCollection<FtReducer>,
    {
        Self {
            command_args: self
                .command_args
                .arg("GROUPBY")
                .arg(properties.num_args())
                .arg(properties)
                .arg(reducers)
                .build(),
        }
    }

    /// Sort the pipeline up until the point of SORTBY, using a list of properties.
    ///
    /// `max` is used to optimized sorting, by sorting only for the n-largest elements.
    /// Although it is not connected to [`limit`](FtAggregateOptions::limit), you usually need just `SORTBY … MAX` for common queries.
    #[must_use]
    pub fn sortby<P>(mut self, properties: P, max: Option<usize>) -> Self
    where
        P: MultipleArgsCollection<FtSortBy>,
    {
        Self {
            command_args: self
                .command_args
                .arg("SORTBY")
                .arg(properties.num_args())
                .arg(properties)
                .arg(max.map(|m| ("MAX", m)))
                .build(),
        }
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
    pub fn apply<E, N>(mut self, expr: E, name: N) -> Self
    where
        E: SingleArg,
        N: SingleArg,
    {
        Self {
            command_args: self
                .command_args
                .arg("APPLY")
                .arg(expr)
                .arg("AS")
                .arg(name)
                .build(),
        }
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
    pub fn limit(mut self, offset: usize, num: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(offset).arg(num).build(),
        }
    }

    /// filters the results using predicate expressions relating to values in each result.
    /// They are applied post query and relate to the current state of the pipeline.
    #[must_use]
    pub fn filter<E, N>(mut self, expr: E) -> Self
    where
        E: SingleArg,
    {
        Self {
            command_args: self.command_args.arg("FILTER").arg(expr).build(),
        }
    }

    /// Scan part of the results with a quicker alternative than [`limit`](FtAggregateOptions::limit).
    /// See [`Cursor API`](https://redis.io/docs/stack/search/reference/aggregations/#cursor-api) for more details.
    #[must_use]
    pub fn withcursor(mut self, options: FtWithCursorOptions) -> Self {
        Self {
            command_args: self.command_args.arg("WITHCURSOR").arg(options).build(),
        }
    }

    /// if set, overrides the timeout parameter of the module.
    #[must_use]
    pub fn timeout(mut self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TIMEOUT").arg(milliseconds).build(),
        }
    }

    /// defines one or more value parameters. Each parameter has a name and a value.
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
    pub fn params<N, V, P>(mut self, params: P) -> Self
    where
        N: SingleArg,
        V: SingleArg,
        P: MultipleArgsCollection<(N, V)>,
    {
        Self {
            command_args: self
                .command_args
                .arg("PARAMS")
                .arg(params.num_args())
                .arg(params)
                .build(),
        }
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(mut self, dialect_version: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("DIALECT")
                .arg(dialect_version)
                .build(),
        }
    }
}

impl ToArgs for FtAggregateOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Attribute for the [`LOAD`](FtAggregateOptions::load) aggregate option
pub struct FtLoadAttribute {
    command_args: CommandArgs,
}

impl FtLoadAttribute {
    #[must_use]
    /// `identifier` is either an attribute name for hashes and JSON or a JSON Path expression for JSON.
    pub fn new<I: SingleArg>(identifier: I) -> Self {
        Self {
            command_args: CommandArgs::default().arg(identifier).build(),
        }
    }

    /// `property` is the optional name used in the result.
    ///
    /// If it is not provided, the identifier is used.
    /// This should be avoided.
    #[must_use]
    pub fn property<P: SingleArg>(property: P) -> Self {
        Self {
            command_args: CommandArgs::default().arg("AS").arg(property).build(),
        }
    }
}

impl ToArgs for FtLoadAttribute {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }

    fn num_args(&self) -> usize {
        self.command_args.len()
    }
}

/// Reducer for the [`groupby`](FtAggregateOptions::groupby) aggregate option
pub struct FtReducer {
    command_args: CommandArgs,
}

impl FtReducer {
    #[must_use]
    /// Count the number of records in each group
    pub fn count() -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("COUNT")
                .arg(0)
                .build(),
        }
    }

    /// Count the number of distinct values for property.
    ///
    /// # Note
    /// The reducer creates a hash-set per group, and hashes each record.
    /// This can be memory heavy if the groups are big.
    pub fn count_distinct<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("COUNT_DISTINCT")
                .arg(1)
                .arg(property)
                .build(),
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
    pub fn count_distinctish<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("COUNT_DISTINCTISH")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the sum of all numeric values of a given property in a group.
    ///
    /// Non numeric values if the group are counted as 0.
    pub fn sum<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("SUM")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the minimal value of a property, whether it is a string, number or NULL.
    pub fn min<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("MIN")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the maximal value of a property, whether it is a string, number or NULL.
    pub fn max<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("MAX")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the average value of a numeric property.
    ///
    /// This is equivalent to reducing by sum and count,
    /// and later on applying the ratio of them as an APPLY step.
    pub fn avg<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("AVG")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the [`standard deviation`](https://en.wikipedia.org/wiki/Standard_deviation)
    /// of a numeric property in the group.
    pub fn stddev<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("STDDEV")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the value of a numeric property at a given quantile of the results.
    ///
    /// Quantile is expressed as a number between 0 and 1.
    /// For example, the median can be expressed as the quantile at 0.5, e.g. REDUCE QUANTILE 2 @foo 0.5 AS median .
    /// If multiple quantiles are required, just repeat the QUANTILE reducer for each quantile.
    /// e.g. REDUCE QUANTILE 2 @foo 0.5 AS median REDUCE QUANTILE 2 @foo 0.99 AS p99
    pub fn quantile<P: SingleArg>(property: P, quantile: f64) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("QUANTILE")
                .arg(2)
                .arg(property)
                .arg(quantile)
                .build(),
        }
    }

    /// Merge all `distinct` values of a given property into a single array.
    pub fn tolist<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("TOLIST")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    ///
    /// If no BY is specified, we return the first value we encounter in the group.
    /// If you with to get the top or bottom value in the group sorted by the same value,
    /// you are better off using the MIN/MAX reducers,
    /// but the same effect will be achieved by doing REDUCE FIRST_VALUE 4 @foo BY @foo DESC.
    pub fn first_value<P: SingleArg>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("FIRST_VALUE")
                .arg(1)
                .arg(property)
                .build(),
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    pub fn first_value_by<P: SingleArg, BP: SingleArg>(property: P, by_property: BP) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("FIRST_VALUE")
                .arg(2)
                .arg(property)
                .arg(by_property)
                .build(),
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    pub fn first_value_by_order<P: SingleArg, BP: SingleArg>(
        property: P,
        by_property: BP,
        order: SortOrder,
    ) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("FIRST_VALUE")
                .arg(3)
                .arg(property)
                .arg(by_property)
                .arg(order)
                .build(),
        }
    }

    /// Perform a reservoir sampling of the group elements with a given size,
    ///  and return an array of the sampled items with an even distribution.
    pub fn random_sample<P: SingleArg, BP: SingleArg>(
        property: P,
        sample_size: usize,
    ) -> FtReducer {
        Self {
            command_args: CommandArgs::default()
                .arg("REDUCE")
                .arg("RANDOM_SAMPLE")
                .arg(2)
                .arg(property)
                .arg(sample_size)
                .build(),
        }
    }

    /// The reducers can have their own property names using the AS {name} optional argument.
    ///
    /// If a name is not given, the resulting name will be
    /// the name of the reduce function and the group properties.
    /// For example, if a name is not given to COUNT_DISTINCT by property @foo,
    /// the resulting name will be count_distinct(@foo).
    pub fn as_name<N: SingleArg>(mut self, name: N) -> Self {
        Self {
            command_args: self.command_args.arg("AS").arg(name).build(),
        }
    }
}

impl ToArgs for FtReducer {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// option for the [`sortby`](FtAggregateOptions::sortby) aggregate option
pub struct FtSortBy {
    command_args: CommandArgs,
}

impl FtSortBy {
    /// sort by property
    pub fn property<P: SingleArg>(property: P) -> FtSortBy {
        Self {
            command_args: CommandArgs::default().arg(property).build(),
        }
    }

    /// ascending
    pub fn asc(mut self) -> FtSortBy {
        Self {
            command_args: self.command_args.arg("ASC").build(),
        }
    }

    /// ascending
    pub fn desc(mut self) -> FtSortBy {
        Self {
            command_args: self.command_args.arg("DESC").build(),
        }
    }
}

impl ToArgs for FtSortBy {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }

    fn num_args(&self) -> usize {
        self.command_args.len()
    }
}

/// options for the [`withcursor`](FtAggregateOptions::withcursor) aggregate option
#[derive(Default)]
pub struct FtWithCursorOptions {
    command_args: CommandArgs,
}

impl FtWithCursorOptions {
    /// Control how many rows are read per each cursor fetch.
    pub fn count(mut self, read_size: usize) -> FtWithCursorOptions {
        Self {
            command_args: self.command_args.arg("COUNT").arg(read_size).build(),
        }
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
        Self {
            command_args: self.command_args.arg("MAXIDLE").arg(idle_time_ms).build(),
        }
    }
}

impl ToArgs for FtWithCursorOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Result for the [`ft_aggregate`](SearchCommands::ft_aggregate) command
#[derive(Debug)]
pub struct FtAggregateResult {
    pub total_results: usize,
    pub results: Vec<Vec<(String, String)>>,
    pub cursor_id: Option<u64>,
}

impl<'de> Deserialize<'de> for FtAggregateResult {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum TotalResultsOrResult {
            TotalResults(usize),
            Result(FtAggregateResult),
        }

        struct TotalResultsOrResultSeed;

        impl<'de> DeserializeSeed<'de> for TotalResultsOrResultSeed {
            type Value = TotalResultsOrResult;

            fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct TotalResultsOrResultVisitor;

                impl<'de> Visitor<'de> for TotalResultsOrResultVisitor {
                    type Value = TotalResultsOrResult;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("TotalResultsOrResults")
                    }

                    fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok(TotalResultsOrResult::TotalResults(v as usize))
                    }

                    fn visit_seq<A>(self, seq: A) -> std::result::Result<Self::Value, A::Error>
                    where
                        A: de::SeqAccess<'de>,
                    {
                        let result =
                            FtAggregateResult::deserialize(SeqAccessDeserializer::new(seq))?;
                        Ok(TotalResultsOrResult::Result(result))
                    }
                }

                deserializer.deserialize_any(TotalResultsOrResultVisitor)
            }
        }

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
                let Some(first) = seq.next_element_seed(TotalResultsOrResultSeed)? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                match first {
                    TotalResultsOrResult::TotalResults(total_results) => {
                        let mut results = if let Some(size) = seq.size_hint() {
                            Vec::<Vec<(String, String)>>::with_capacity(size - 1)
                        } else {
                            Vec::<Vec<(String, String)>>::new()
                        };

                        while let Some(sub_results) =
                            seq.next_element_seed(VecOfPairsSeed::<String, String>::new())?
                        {
                            results.push(sub_results);
                        }

                        Ok(FtAggregateResult {
                            total_results,
                            results,
                            cursor_id: None,
                        })
                    }
                    TotalResultsOrResult::Result(mut result) => {
                        let Some(cursor_id) = seq.next_element::<u64>()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };

                        result.cursor_id = Some(cursor_id);
                        Ok(result)
                    }
                }
            }
        }

        deserializer.deserialize_seq(FtAggregateResultVisitor)
    }
}

/// Result for the [`ft_search`](SearchCommands::ft_search) command
#[derive(Debug)]
pub struct FtSearchResult {
    pub total_results: usize,
    pub results: Vec<FtSearchResultRow>,
}

/// A row in a [`FtSearchResult`](FtSearchResult)
#[derive(Debug, Default)]
pub struct FtSearchResultRow {
    /// Will be empty for [`ft_aggregate`](SearchCommands::ft_aggregate)
    pub document_id: String,
    /// relative internal score of each document. only if [`withscores`](FtSearchOptions::withscores) is set
    pub score: f64,
    /// document payload. only if [`withpayloads`](FtSearchOptions::withpayloads) is set
    pub payload: Vec<u8>,
    /// value of the sorting key. only if [`withsortkeys`](FtSearchOptions::withsortkeys) is set
    pub sortkey: String,
    /// collection of attribute/value pairs.
    pub values: Vec<(String, String)>,
}

impl<'de> Deserialize<'de> for FtSearchResult {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        enum RowField {
            DocumentId(String),
            Score(f64),
            Payload(Vec<u8>),
            Sortkey(String),
            Values(Vec<(String, String)>),
        }

        enum RowSeedState {
            Start,
            AfterDocumentId(usize),
        }

        struct RowSeed {
            row_num_fields: usize,
            state: RowSeedState,
        }

        impl RowSeed {
            pub fn new(row_num_fields: usize) -> Self {
                Self {
                    row_num_fields,
                    state: RowSeedState::Start,
                }
            }
        }

        impl<'de> DeserializeSeed<'de> for &mut RowSeed {
            type Value = RowField;

            #[inline]
            fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(self)
            }
        }

        impl<'de> Visitor<'de> for &mut RowSeed {
            type Value = RowField;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("RowField")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                match self.state {
                    RowSeedState::Start => {
                        if self.row_num_fields > 1 {
                            self.state = RowSeedState::AfterDocumentId(1);
                        }
                        Ok(RowField::DocumentId(
                            std::str::from_utf8(v)
                                .map_err(de::Error::custom)?
                                .to_owned(),
                        ))
                    }
                    RowSeedState::AfterDocumentId(field_index) => {
                        if field_index == self.row_num_fields - 1 {
                            self.state = RowSeedState::Start;
                        } else {
                            self.state = RowSeedState::AfterDocumentId(field_index + 1);
                        }

                        // sortkeys begin by a '$' char
                        if let Some(b'$') = v.first() {
                            Ok(RowField::Sortkey(
                                std::str::from_utf8(v)
                                    .map_err(de::Error::custom)?
                                    .to_owned(),
                            ))
                        } else {
                            Ok(RowField::Payload(v.to_vec()))
                        }
                    }
                }
            }

            fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                match self.state {
                    RowSeedState::Start => unreachable!(),
                    RowSeedState::AfterDocumentId(field_index) => {
                        if field_index == self.row_num_fields - 1 {
                            self.state = RowSeedState::Start;
                        } else {
                            self.state = RowSeedState::AfterDocumentId(field_index + 1);
                        }

                        Ok(RowField::Score(v))
                    }
                }
            }

            fn visit_seq<A>(self, seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                self.state = RowSeedState::Start;
                let values = deserialize_vec_of_pairs(SeqAccessDeserializer::new(seq))?;
                Ok(RowField::Values(values))
            }
        }

        struct FtSearchResultVisitor;

        impl<'de> Visitor<'de> for FtSearchResultVisitor {
            type Value = FtSearchResult;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("FtSearchResult")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(total_results) = seq.next_element()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let Some(seq_size) = seq.size_hint() else {
                    return Err(de::Error::custom("sequence `size_hint` is expected for FtSearchResult"));
                };

                let mut results = Vec::with_capacity(total_results);

                if seq_size > 0 {
                    let row_num_fields = (seq_size - 1) / total_results;
                    let mut row: Option<FtSearchResultRow> = None;
                    let mut row_seed = RowSeed::new(row_num_fields);

                    while let Some(item) = seq.next_element_seed(&mut row_seed)? {
                        match item {
                            RowField::DocumentId(document_id) => {
                                if let Some(row) = row.take() {
                                    results.push(row);
                                }
                                row = Some(FtSearchResultRow {
                                    document_id,
                                    ..Default::default()
                                })
                            }
                            RowField::Score(score) => {
                                if let Some(row) = &mut row {
                                    row.score = score;
                                }
                            }
                            RowField::Payload(payload) => {
                                if let Some(row) = &mut row {
                                    row.payload = payload;
                                }
                            }
                            RowField::Sortkey(sortkey) => {
                                if let Some(row) = &mut row {
                                    row.sortkey = sortkey;
                                }
                            }
                            RowField::Values(values) => {
                                if let Some(row) = &mut row {
                                    row.values = values;
                                }
                            }
                        }
                    }

                    if let Some(row) = row.take() {
                        results.push(row);
                    }
                }

                Ok(FtSearchResult {
                    total_results,
                    results,
                })
            }
        }

        deserializer.deserialize_seq(FtSearchResultVisitor)
    }
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
    pub max_doc_id: String,
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
    pub number_of_uses: usize,
    #[serde(default)]
    pub gc_stats: Option<FtGcStats>,
    #[serde(default)]
    pub cursor_stats: Option<FtCursorStats>,
    /// if a custom stopword list is used.
    #[serde(default)]
    pub stopwords_list: Vec<String>,
}

/// Index attribute info
#[derive(Debug, Default )]
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
        struct FtIndexAttributeVisitor;

        impl<'de> Visitor<'de> for FtIndexAttributeVisitor {
            type Value = FtIndexAttribute;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("FtIndexAttribute")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut attribute = FtIndexAttribute::default();

                while let Some(field_name) = seq.next_element::<&str>()? {
                    match field_name {
                        "identifier" => {
                            if let Some(identifier) = seq.next_element::<String>()? {
                                attribute.identifier = identifier;
                            }
                        }
                        "attribute" => {
                            if let Some(alias) = seq.next_element::<String>()? {
                                attribute.attribute = alias;
                            }
                        }
                        "type" => {
                            if let Some(field_type) = seq.next_element::<FtFieldType>()? {
                                attribute.field_type = field_type;
                            }
                        }
                        "WEIGHT" => {
                            if let Some(weight) = seq.next_element::<f64>()? {
                                attribute.weight = weight;
                            }
                        }
                        "SORTABLE" => attribute.sortable = true,
                        "UNF" => attribute.unf = true,
                        "NOSTEM" => attribute.no_stem = true,
                        "NOINDEX" => attribute.no_index = true,
                        "PHONETIC" => {
                            attribute.phonetic = seq.next_element::<FtPhoneticMatcher>()?
                        }
                        "SEPARATOR" => attribute.separator = seq.next_element::<char>()?,
                        "CASESENSITIVE" => attribute.case_sensitive = true,
                        "WITHSUFFIXTRIE" => attribute.with_suffixe_trie = true,
                        _ => (),
                    }
                }

                Ok(attribute)
            }
        }

        deserializer.deserialize_seq(FtIndexAttributeVisitor)
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
}

/// Result for the [`ft_profile_search`](SearchCommands::ft_profile_search) command.
#[derive(Debug, Deserialize)]
pub struct FtProfileSearchResult {
    pub results: FtSearchResult,
    pub profile_details: FtProfileDetails,
}

/// Result for the [`ft_profile_aggregate`](SearchCommands::ft_profile_aggregate) command.
#[derive(Debug, Deserialize)]
pub struct FtProfileAggregateResult {
    pub results: FtAggregateResult,
    pub profile_details: FtProfileDetails,
}

/// Result details of a [`ft_profile_search`](SearchCommands::ft_profile_search)
/// or [`ft_profile_aggregate`](SearchCommands::ft_profile_aggregate) command.
#[derive(Debug)]
pub struct FtProfileDetails {
    /// The total runtime of the query.
    pub total_profile_time: f64,
    /// Parsing time of the query and parameters into an execution plan.
    pub parsing_time: f64,
    /// Creation time of execution plan including iterators, result processors and reducers creation.
    pub pipeline_creation_time: f64,
    ///  Index iterators information including their type, term, count, and time data.
    ///
    /// Inverted-index iterators have in addition the number of elements they contain.
    /// Hybrid vector iterators returning the top results from the vector index in batches,
    /// include the number of batches.
    pub iterators_profile: HashMap<String, Value>,
    /// Result processors chain with type, count and time data.
    pub result_processors_profile: Vec<FtResultProcessorsProfile>,
}

impl<'de> Deserialize<'de> for FtProfileDetails {
    #[inline]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum FtProfileDetailsField {
            TotalProfileTime(f64),
            ParsingTime(f64),
            PipelineCreationTime(f64),
            IteratorsProfile(HashMap<String, Value>),
            ResultProcessorsProfile(Vec<FtResultProcessorsProfile>),
        }

        struct FtProfileDetailsFieldVisitor;

        impl<'de> Visitor<'de> for FtProfileDetailsFieldVisitor {
            type Value = FtProfileDetailsField;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("FtProfileDetailsField")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(field) = seq.next_element::<&str>()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                match field {
                    "Total profile time" => {
                        let Some(value) = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        Ok(FtProfileDetailsField::TotalProfileTime(value))
                    }
                    "Parsing time" => {
                        let Some(value) = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        Ok(FtProfileDetailsField::ParsingTime(value))
                    }
                    "Pipeline creation time" => {
                        let Some(value) = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        Ok(FtProfileDetailsField::PipelineCreationTime(value))
                    }
                    "Iterators profile" => {
                        let Some(value) = seq.next_element()? else {
                            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                        };
                        Ok(FtProfileDetailsField::IteratorsProfile(value))
                    }
                    "Result processors profile" => {
                        let mut results = if let Some(size_hint) = seq.size_hint() {
                            Vec::with_capacity(size_hint)
                        } else {
                            Vec::new()
                        };

                        while let Some(result) = seq.next_element()? {
                            results.push(result);
                        }

                        Ok(FtProfileDetailsField::ResultProcessorsProfile(results))
                    }
                    _ => Err(de::Error::unknown_field(field, &[])),
                }
            }
        }

        struct FtProfileDetailsFieldSeed;

        impl<'de> DeserializeSeed<'de> for FtProfileDetailsFieldSeed {
            type Value = FtProfileDetailsField;

            #[inline]
            fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_seq(FtProfileDetailsFieldVisitor)
            }
        }

        struct FtProfileDetailsVisitor;

        impl<'de> Visitor<'de> for FtProfileDetailsVisitor {
            type Value = FtProfileDetails;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("FtProfileDetails")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut total_profile_time = None;
                let mut parsing_time = None;
                let mut pipeline_creation_time = None;
                let mut iterators_profile = None;
                let mut result_processors_profile = None;

                while let Some(field) = seq.next_element_seed(FtProfileDetailsFieldSeed)? {
                    match field {
                        FtProfileDetailsField::TotalProfileTime(v) => total_profile_time = Some(v),
                        FtProfileDetailsField::ParsingTime(v) => parsing_time = Some(v),
                        FtProfileDetailsField::PipelineCreationTime(v) => {
                            pipeline_creation_time = Some(v)
                        }
                        FtProfileDetailsField::IteratorsProfile(v) => iterators_profile = Some(v),
                        FtProfileDetailsField::ResultProcessorsProfile(v) => {
                            result_processors_profile = Some(v)
                        }
                    }
                }

                let total_profile_time = total_profile_time
                    .ok_or_else(|| de::Error::missing_field("total_profile_time"))?;
                let parsing_time =
                    parsing_time.ok_or_else(|| de::Error::missing_field("parsing_time"))?;
                let pipeline_creation_time = pipeline_creation_time
                    .ok_or_else(|| de::Error::missing_field("pipeline_creation_time"))?;
                let iterators_profile = iterators_profile
                    .ok_or_else(|| de::Error::missing_field("iterators_profile"))?;
                let result_processors_profile = result_processors_profile
                    .ok_or_else(|| de::Error::missing_field("result_processors_profile"))?;

                Ok(FtProfileDetails {
                    total_profile_time,
                    parsing_time,
                    pipeline_creation_time,
                    iterators_profile,
                    result_processors_profile,
                })
            }
        }

        deserializer.deserialize_seq(FtProfileDetailsVisitor)
    }
}

/// Result processors profile for the [`ft_profile_search`](SearchCommands::ft_profile_search)
/// or [`ft_profile_aggregate`](SearchCommands::ft_profile_aggregate) command.
#[derive(Debug, Deserialize)]
pub struct FtResultProcessorsProfile {
    #[serde(rename = "Type")]
    pub _type: String,
    #[serde(rename = "Time")]
    pub time: f64,
    #[serde(rename = "Counter")]
    pub counter: usize,
}

/// Options for the [`ft_search`](SearchCommands::ft_search) command.
#[derive(Default)]
pub struct FtSearchOptions {
    command_args: CommandArgs,
}

impl FtSearchOptions {
    /// returns the document ids and not the content.
    /// This is useful if RediSearch is only an index on an external document collection.
    #[must_use]
    pub fn nocontent(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOCONTENT").build(),
        }
    }

    /// does not try to use stemming for query expansion but searches the query terms verbatim.
    #[must_use]
    pub fn verbatim(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("VERBATIM").build(),
        }
    }

    /// also returns the relative internal score of each document.
    ///
    /// This can be used to merge results from multiple instances.
    #[must_use]
    pub fn withscores(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSCORES").build(),
        }
    }

    /// retrieves optional document payloads.
    ///
    /// See [`ft_create`](SearchCommands::ft_create)
    /// The payloads follow the document id and, if [`withscores`](FtSearchOptions::withscores) is set, the scores.
    #[must_use]
    pub fn withpayloads(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHPAYLOADS").build(),
        }
    }

    /// returns the value of the sorting key, right after the id and score and/or payload, if requested.
    ///
    /// This is usually not needed, and exists for distributed search coordination purposes.
    /// This option is relevant only if used in conjunction with [`sortby`](FtSearchOptions::sortby).
    #[must_use]
    pub fn withsortkeys(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSORTKEYS").build(),
        }
    }

    /// limits results to those having numeric values ranging between min and max,
    /// if numeric_field is defined as a numeric field in [`ft_create`](SearchCommands::ft_create).
    ///
    /// `min` and `max` follow [`zrange`](crate::commands::SortedSetCommands::zrange) syntax, and can be `-inf`, `+inf`,
    /// and use `(` for exclusive ranges. Multiple numeric filters for different attributes are supported in one query.
    #[must_use]
    pub fn filter(
        mut self,
        numeric_field: impl SingleArg,
        min: impl SingleArg,
        max: impl SingleArg,
    ) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTER")
                .arg(numeric_field)
                .arg(min)
                .arg(max)
                .build(),
        }
    }

    /// filter the results to a given `radius` from `lon` and `lat`.
    ///
    /// `radius` is given as a number and units.
    /// See [`geosearch`](crate::commands::GeoCommands::geosearch) for more details.
    #[must_use]
    pub fn geo_filter(
        mut self,
        geo_field: impl SingleArg,
        lon: f64,
        lat: f64,
        radius: f64,
        unit: GeoUnit,
    ) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("GEOFILTER")
                .arg(geo_field)
                .arg(lon)
                .arg(lat)
                .arg(radius)
                .arg(unit)
                .build(),
        }
    }

    /// limits the result to a given set of keys specified in the list.
    ///
    /// Non-existent keys are ignored, unless all the keys are non-existent.
    #[must_use]
    pub fn inkeys<A>(mut self, keys: impl SingleArgCollection<A>) -> Self
    where
        A: SingleArg,
    {
        Self {
            command_args: self
                .command_args
                .arg("INKEYS")
                .arg(keys.num_args())
                .arg(keys)
                .build(),
        }
    }

    /// filters the results to those appearing only in specific attributes of the document, like `title` or `URL`.
    #[must_use]
    pub fn infields<A>(mut self, attributes: impl SingleArgCollection<A>) -> Self
    where
        A: SingleArg,
    {
        Self {
            command_args: self
                .command_args
                .arg("INFIELDS")
                .arg(attributes.num_args())
                .arg(attributes)
                .build(),
        }
    }

    /// limits the attributes returned from the document.
    ///
    /// If attributes is empty, it acts like [`nocontent`](FtSearchOptions::nocontent).
    #[must_use]
    pub fn _return(
        mut self,
        attributes: impl MultipleArgsCollection<FtSearchReturnAttribute>,
    ) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("RETURN")
                .arg(attributes.num_args())
                .arg(attributes)
                .build(),
        }
    }

    /// returns only the sections of the attribute that contain the matched text.
    ///
    /// See [`Highlighting`](https://redis.io/docs/stack/search/reference/highlight) for more information.
    #[must_use]
    pub fn summarize(mut self, options: FtSearchSummarizeOptions) -> Self {
        Self {
            command_args: self.command_args.arg("SUMMARIZE").arg(options).build(),
        }
    }

    /// formats occurrences of matched text.
    ///
    /// See [`Highlighting`](https://redis.io/docs/stack/search/reference/highlight) for more information.
    #[must_use]
    pub fn highlight(mut self, options: FtSearchHighlightOptions) -> Self {
        Self {
            command_args: self.command_args.arg("HIGHLIGHT").arg(options).build(),
        }
    }

    /// allows a maximum of N intervening number of unmatched offsets between phrase terms.
    ///
    /// In other words, the slop for exact phrases is 0.
    #[must_use]
    pub fn slop(mut self, slop: usize) -> Self {
        Self {
            command_args: self.command_args.arg("SLOP").arg(slop).build(),
        }
    }

    /// puts the query terms in the same order in the document as in the query,
    /// regardless of the offsets between them.
    ///
    /// Typically used in conjunction with [`slop`](FtSearchOptions::slop).
    #[must_use]
    pub fn inorder(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("INORDER").build(),
        }
    }

    /// use a stemmer for the supplied language during search for query expansion.
    ///
    /// If querying documents in Chinese, set to chinese to properly tokenize the query terms.
    /// Defaults to English.
    /// If an unsupported language is sent, the command returns an error.
    /// See FT.CREATE for the list of languages.
    #[must_use]
    pub fn language(mut self, language: FtLanguage) -> Self {
        Self {
            command_args: self.command_args.arg("LANGUAGE").arg(language).build(),
        }
    }

    /// uses a custom query `expander` instead of the stemmer.
    ///
    /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn expander(mut self, expander: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("EXPANDER").arg(expander).build(),
        }
    }

    /// uses a custom scoring function you define.
    ///
    /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn scorer(mut self, scorer: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("SCORER").arg(scorer).build(),
        }
    }

    /// returns a textual description of how the scores were calculated.
    ///
    /// Using this options requires the [`withscores`](FtSearchOptions::withscores) option.
    #[must_use]
    pub fn explainscore(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("EXPLAINSCORE").build(),
        }
    }

    /// adds an arbitrary, binary safe `payload` that is exposed to custom scoring functions.
    ///
    /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn payload(mut self, payload: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("PAYLOAD").arg(payload).build(),
        }
    }

    /// orders the results by the value of this attribute.
    ///
    /// This applies to both text and numeric attributes.
    /// Attributes needed for `sortby` should be declared as [`SORTABLE`](FtFieldSchema::sortable) in the index,
    /// in order to be available with very low latency. Note that this adds memory overhead.
    #[must_use]
    pub fn sortby(mut self, attribute: impl SingleArg, order: SortOrder) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("SORTBY")
                .arg(attribute)
                .arg(order)
                .build(),
        }
    }

    /// limits the results to the offset and number of results given.
    ///
    /// Note that the offset is zero-indexed. The default is `0 10`, which returns 10 items starting from the first result.
    /// You can use `LIMIT 0 0` to count the number of documents in the result set without actually returning them.
    #[must_use]
    pub fn limit(mut self, first: usize, num: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(first).arg(num).build(),
        }
    }

    /// overrides the timeout parameter of the module.
    #[must_use]
    pub fn timeout(mut self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TIMEOUT").arg(milliseconds).build(),
        }
    }

    /// defines one or more value parameters. Each parameter has a name and a value.
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
    pub fn params<N, V, P>(mut self, params: P) -> Self
    where
        N: SingleArg,
        V: SingleArg,
        P: MultipleArgsCollection<(N, V)>,
    {
        Self {
            command_args: self
                .command_args
                .arg("PARAMS")
                .arg(params.num_args())
                .arg(params)
                .build(),
        }
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(mut self, dialect_version: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("DIALECT")
                .arg(dialect_version)
                .build(),
        }
    }
}

impl ToArgs for FtSearchOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// attribute for the [`search`](SearchCommands::ft_search) option [`return`](FtSearchOptions::_return)
pub struct FtSearchReturnAttribute {
    command_args: CommandArgs,
}

impl FtSearchReturnAttribute {
    /// `identifier`is either an attribute name (for hashes and JSON) or a JSON Path expression (for JSON).
    #[must_use]
    pub fn identifier(identifier: impl SingleArg) -> Self {
        Self {
            command_args: CommandArgs::default().arg(identifier).build(),
        }
    }

    /// `property`is an optional name used in the result.
    ///
    /// If not provided, the `identifier` is used in the result.
    #[must_use]
    pub fn as_property(mut self, property: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("AS").arg(property).build(),
        }
    }
}

impl ToArgs for FtSearchReturnAttribute {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// sub-options for the [`search`](SearchCommands::ft_search) option [`summarize`](FtSearchOptions::summarize)
#[derive(Default)]
pub struct FtSearchSummarizeOptions {
    command_args: CommandArgs,
}

impl FtSearchSummarizeOptions {
    /// If present, must be the first argument.
    /// Each field present is summarized.
    /// If no `FIELDS` directive is passed, then all fields returned are summarized.
    #[must_use]
    pub fn fields<F: SingleArg>(mut self, fields: impl SingleArgCollection<F>) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields)
                .build(),
        }
    }

    /// How many fragments should be returned. If not specified, a default of 3 is used.
    #[must_use]
    pub fn frags(mut self, num_frags: usize) -> Self {
        Self {
            command_args: self.command_args.arg("FRAGS").arg(num_frags).build(),
        }
    }

    /// The number of context words each fragment should contain.
    ///
    /// Context words surround the found term.
    /// A higher value will return a larger block of text.
    /// If not specified, the default value is 20.
    #[must_use]
    pub fn len(mut self, frag_len: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LEN").arg(frag_len).build(),
        }
    }

    /// The string used to divide between individual summary snippets.
    ///
    /// The default is `...` which is common among search engines;
    /// but you may override this with any other string if you desire to programmatically divide them later on.
    /// You may use a newline sequence, as newlines are stripped from the result body anyway
    /// (thus, it will not be conflated with an embedded newline in the text)
    #[must_use]
    pub fn separator(mut self, separator: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("SEPARATOR").arg(separator).build(),
        }
    }
}

impl ToArgs for FtSearchSummarizeOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// sub-options for the [`search`](SearchCommands::ft_search) option [`summarize`](FtSearchOptions::highlight)
#[derive(Default)]
pub struct FtSearchHighlightOptions {
    command_args: CommandArgs,
}

impl FtSearchHighlightOptions {
    /// If present, must be the first argument.
    /// Each field present is highlighted.
    /// If no `FIELDS` directive is passed, then all fields returned are highlighted.
    #[must_use]
    pub fn fields<F: SingleArg>(mut self, fields: impl SingleArgCollection<F>) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FIELDS")
                .arg(fields.num_args())
                .arg(fields)
                .build(),
        }
    }

    /// * `open_tag` - prepended to each term match
    /// * `close_tag` - appended to each term match
    /// If no `TAGS` are specified, a built-in tag value is appended and prepended.
    #[must_use]
    pub fn tags(mut self, open_tag: impl SingleArg, close_tag: impl SingleArg) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("TAGS")
                .arg(open_tag)
                .arg(close_tag)
                .build(),
        }
    }
}

impl ToArgs for FtSearchHighlightOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Redis search supported languages
/// See. [`Supported Languages`](https://redis.io/docs/stack/search/reference/stemming/#supported-languages)
pub enum FtLanguage {
    Arabic,
    Armenian,
    Basque,
    Catalan,
    Chinese,
    Danish,
    Dutch,
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

impl Default for FtLanguage {
    fn default() -> Self {
        Self::English
    }
}

impl ToArgs for FtLanguage {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            FtLanguage::Arabic => "arabic",
            FtLanguage::Armenian => "armenian",
            FtLanguage::Basque => "basque",
            FtLanguage::Catalan => "catalan",
            FtLanguage::Chinese => "chinese",
            FtLanguage::Danish => "danish",
            FtLanguage::Dutch => "dutch",
            FtLanguage::English => "english",
            FtLanguage::Finnish => "finnish",
            FtLanguage::French => "french",
            FtLanguage::German => "german",
            FtLanguage::Greek => "greek",
            FtLanguage::Hungarian => "hungarian",
            FtLanguage::Indonesian => "indonesian",
            FtLanguage::Irish => "irish",
            FtLanguage::Italian => "italian",
            FtLanguage::Lithuanian => "lithuanian",
            FtLanguage::Nepali => "nepali",
            FtLanguage::Norwegian => "norwegian",
            FtLanguage::Portuguese => "portuguese",
            FtLanguage::Romanian => "romanian",
            FtLanguage::Russian => "russian",
            FtLanguage::Serbian => "serbian",
            FtLanguage::Spanish => "spanish",
            FtLanguage::Swedish => "swedish",
            FtLanguage::Tamil => "tamil",
            FtLanguage::Turkish => "turkish",
            FtLanguage::Yiddish => "yiddish",
        });
    }
}

/// Options for the [`ft_spellcheck`](SearchCommands::ft_spellcheck) command.
#[derive(Default)]
pub struct FtSpellCheckOptions {
    command_args: CommandArgs,
}

impl FtSpellCheckOptions {
    /// maximum Levenshtein distance for spelling suggestions (default: 1, max: 4).
    #[must_use]
    pub fn distance(mut self, distance: u64) -> Self {
        Self {
            command_args: self.command_args.arg("DISTANCE").arg(distance).build(),
        }
    }

    /// specifies an inclusion (`FtTermType::Include`) or exclusion (`FtTermType::Exclude`) of a custom dictionary named `dictionary`
    ///
    /// Refer to [`ft_dictadd`](SearchCommands::ft_dictadd), [`ft_dictdel`](SearchCommands::ft_dictdel)
    /// and [`ft_dictdump`](SearchCommands::ft_dictdump) about managing custom dictionaries.
    #[must_use]
    pub fn terms(mut self, term_type: FtTermType, dictionary: impl SingleArg) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("TERMS")
                .arg(term_type)
                .arg(dictionary)
                .build(),
        }
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(mut self, dialect_version: u64) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("DIALECT")
                .arg(dialect_version)
                .build(),
        }
    }
}

impl ToArgs for FtSpellCheckOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Term type for the option [`terms`](FtSpellCheckOptions::terms)
pub enum FtTermType {
    Include,
    Exclude,
}

impl ToArgs for FtTermType {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(match self {
            FtTermType::Include => "INCLUDE",
            FtTermType::Exclude => "EXCLUDE",
        });
    }
}

/// Result for the [`ft_spellcheck`](SearchCommands::ft_spellcheck) command.
#[derive(Debug)]
pub struct FtSpellCheckResult {
    /// a collection where each element represents a misspelled term from the query + suggestions for this term
    ///
    /// The misspelled terms are ordered by their order of appearance in the query.
    pub misspelled_terms: Vec<FtMisspelledTerm>,
}

impl<'de> Deserialize<'de> for FtSpellCheckResult {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(FtSpellCheckResult {
            misspelled_terms: Vec::<FtMisspelledTerm>::deserialize(deserializer)?,
        })
    }
}

/// Misspelled term + suggestions for the [`ft_spellcheck`](SearchCommands::ft_spellcheck) command.
#[derive(Debug)]
pub struct FtMisspelledTerm {
    /// Misspelled term
    pub misspelled_term: String,
    /// Suggestion as a tuple composed of
    /// * the score of the suggestion
    /// * the suggestion itself
    pub suggestions: Vec<(f64, String)>,
}

impl<'de> Deserialize<'de> for FtMisspelledTerm {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (field_name, misspelled_term, suggestions) =
            <(String, String, Vec<(f64, String)>)>::deserialize(deserializer)?;
        if field_name != "TERM" {
            return Err(de::Error::unknown_field(field_name.as_str(), &["TERM"]));
        }

        Ok(FtMisspelledTerm {
            misspelled_term,
            suggestions,
        })
    }
}

/// Options for the [`ft_sugadd`](SearchCommands::ft_sugadd) command.
#[derive(Default)]
pub struct FtSugAddOptions {
    command_args: CommandArgs,
}

impl FtSugAddOptions {
    /// increments the existing entry of the suggestion by the given score, instead of replacing the score.
    ///
    /// This is useful for updating the dictionary based on user queries in real time.
    #[must_use]
    pub fn incr(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("INCR").build(),
        }
    }

    /// saves an extra payload with the suggestion
    #[must_use]
    pub fn payload(mut self, payload: impl SingleArg) -> Self {
        Self {
            command_args: self.command_args.arg("PAYLOAD").arg(payload).build(),
        }
    }
}

impl ToArgs for FtSugAddOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
    }
}

/// Options for the [`ft_sugget`](SearchCommands::ft_sugget) command.
#[derive(Default)]
pub struct FtSugGetOptions {
    command_args: CommandArgs,
}

impl FtSugGetOptions {
    /// performs a fuzzy prefix search, including prefixes at Levenshtein distance of 1 from the prefix sent.
    #[must_use]
    pub fn fuzzy(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("INCR").build(),
        }
    }

    /// limits the results to a maximum of `num` (default: 5).
    #[must_use]
    pub fn max(mut self, num: usize) -> Self {
        Self {
            command_args: self.command_args.arg("MAX").arg(num).build(),
        }
    }

    /// returns the score of each suggestion.
    ///
    /// This can be used to merge results from multiple instances.
    #[must_use]
    pub fn withscores(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSCORES").build(),
        }
    }

    /// returns optional payloads saved along with the suggestions.
    ///
    /// If no payload is present for an entry, it returns a null reply.
    #[must_use]
    pub fn withpayload(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHPAYLOADS").build(),
        }
    }
}

impl ToArgs for FtSugGetOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        args.arg(&self.command_args);
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
                let with_scores = self
                    .command
                    .args
                    .iter()
                    .any(|a| a.as_slice() == b"WITHSCORES");
                let with_payloads = self
                    .command
                    .args
                    .iter()
                    .any(|a| a.as_slice() == b"WITHPAYLOADS");

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
