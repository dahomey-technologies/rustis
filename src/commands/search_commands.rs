use std::collections::HashMap;

use crate::{
    prepare_command,
    resp::{
        cmd, ArgsOrCollection, Array, BulkString, CommandArgs, FromKeyValueValueArray,
        FromSingleValueArray, FromValue, HashMapExt, IntoArgs, IntoValueIterator,
        SingleArgOrCollection, Value, Command,
    },
    Error, PreparedCommand, Result, SortOrder, GeoUnit,
};

/// A group of Redis commands related to [`RedisSearch`](https://redis.io/docs/stack/search/)
///
/// # See Also
/// [RedisSearch Commands](https://redis.io/commands/?group=search)
pub trait SearchCommands {
    /// Run a search query on an index,
    /// and perform aggregate transformations on the results,
    /// extracting statistics etc from them
    ///
    /// # Arguments
    /// * `index` - index against which the query is executed.
    /// * `query`- is base filtering query that retrieves the documents.\
    ///  It follows the exact same syntax as the search query,\
    ///  including filters, unions, not, optional, and so on.
    /// * `options` - See [`FtAggregateOptions`](crate::FtAggregateOptions)
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.aggregate/>](https://redis.io/commands/ft.aggregate/)
    /// * [`RedisSeach Aggregations`](https://redis.io/docs/stack/search/reference/aggregations/)
    #[must_use]
    fn ft_aggregate<I, Q>(
        &mut self,
        index: I,
        query: Q,
        options: FtAggregateOptions,
    ) -> PreparedCommand<Self, FtQueryResult>
    where
        Self: Sized,
        I: Into<BulkString>,
        Q: Into<BulkString>,
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
    fn ft_aliasadd<A, I>(&mut self, alias: A, index: I) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        A: Into<BulkString>,
        I: Into<BulkString>,
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
    fn ft_aliasdel<A>(&mut self, alias: A) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        A: Into<BulkString>,
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
    fn ft_aliasupdate<A, I>(&mut self, alias: A, index: I) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        A: Into<BulkString>,
        I: Into<BulkString>,
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
        &mut self,
        index: I,
        skip_initial_scan: bool,
        attribute: FtFieldSchema,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        I: Into<BulkString>,
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
    fn ft_config_get<O, N, V, R>(&mut self, option: O) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        O: Into<BulkString>,
        N: FromValue,
        V: FromValue,
        R: FromKeyValueValueArray<N, V>,
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
    fn ft_config_set<O, V>(&mut self, option: O, value: V) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        O: Into<BulkString>,
        V: Into<BulkString>,
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
        &mut self,
        index: I,
        options: FtCreateOptions,
        schema: S,
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        I: Into<BulkString>,
        S: ArgsOrCollection<FtFieldSchema>,
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
    fn ft_cursor_del<I>(&mut self, index: I, cursor_id: u64) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        I: Into<BulkString>,
    {
        prepare_command(self, cmd("FT.CURSOR").arg("DEL").arg(index).arg(cursor_id))
    }

    /// Read next results from an existing cursor
    ///
    /// # Arguments
    /// * `index` - index name.
    /// * `cursor_id` - id of the cursor.
    /// * `read_size` - number of results to read. This parameter overrides
    /// [`count`](crate::FtWithCursorOptions::count) specified in [`ft_aggregate`](crate::SearchCommands::ft_aggregate).
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.cursor-read/>](https://redis.io/commands/ft.cursor-read/)
    #[must_use]
    fn ft_cursor_read<I>(
        &mut self,
        index: I,
        cursor_id: u64,
    ) -> PreparedCommand<Self, FtQueryResult>
    where
        Self: Sized,
        I: Into<BulkString>,
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
    fn ft_dictadd<D, T, TT>(&mut self, dict: D, terms: TT) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        D: Into<BulkString>,
        T: Into<BulkString>,
        TT: SingleArgOrCollection<T>,
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
    fn ft_dictdel<D, T, TT>(&mut self, dict: D, terms: TT) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        D: Into<BulkString>,
        T: Into<BulkString>,
        TT: SingleArgOrCollection<T>,
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
    fn ft_dictdump<D, T, TT>(&mut self, dict: D) -> PreparedCommand<Self, TT>
    where
        Self: Sized,
        D: Into<BulkString>,
        T: FromValue,
        TT: FromSingleValueArray<T>,
    {
        prepare_command(self, cmd("FT.DICTDUMP").arg(dict))
    }

    /// Delete an index
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `dd` - drop operation that, if set, deletes the actual document hashes
    ///
    /// # Notes
    /// * By default, `ft_dropindex` does not delete the document hashes associated with the index.
    /// Adding the `dd` option deletes the hashes as well.
    /// * When using `ft_dropindex` with the parameter `dd`, if an index creation is still running
    /// ([`ft_create`](crate::SearchCommands::ft_create) is running asynchronously),
    /// only the document hashes that have already been indexed are deleted.
    /// The document hashes left to be indexed remain in the database.
    /// You can use [`ft_info`](crate::SearchCommands::ft_info) to check the completion of the indexing.
    ///
    /// # Return
    /// the number of new terms that were added.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.dropindex/>](https://redis.io/commands/ft.dropindex/)
    #[must_use]
    fn ft_dropindex<I>(&mut self, index: I, dd: bool) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
        I: Into<BulkString>,
    {
        prepare_command(self, cmd("FT.DROPINDEX").arg(index).arg_if(dd, "DD"))
    }

    /// Return the execution plan for a complex query
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `query` - query string, as if sent to [`ft_search`](crate::SearchCommands::ft_search).
    /// * `dialect_version` - dialect version under which to execute the query. \
    ///  If not specified, the query executes under the default dialect version set during module initial loading\
    ///  or via [`ft_config_set`](crate::SearchCommands::ft_config_set) command.
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
        &mut self,
        index: I,
        query: Q,
        dialect_version: Option<u64>,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        I: Into<BulkString>,
        Q: Into<BulkString>,
        R: FromValue,
    {
        prepare_command(
            self,
            cmd("FT.EXPLAIN").arg(index).arg(query).arg(dialect_version),
        )
    }

    /// Return the execution plan for a complex query but formatted for easier reading without using `redis-cli --raw`
    ///
    /// # Arguments
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `query` - query string, as if sent to [`ft_search`](crate::SearchCommands::ft_search).
    /// * `dialect_version` - dialect version under which to execute the query. \
    ///  If not specified, the query executes under the default dialect version set during module initial loading\
    ///  or via [`ft_config_set`](crate::SearchCommands::ft_config_set) command.
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
        &mut self,
        index: I,
        query: Q,
        dialect_version: Option<u64>,
    ) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
        I: Into<BulkString>,
        Q: Into<BulkString>,
        R: FromValue,
        RR: FromSingleValueArray<R>,
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
    /// * `index` - full-text index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    ///
    /// # Return
    /// an instance of [`FtInfoResult`](crate::FtInfoResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.info/>](https://redis.io/commands/ft.info/)
    #[must_use]
    fn ft_info(&mut self, index: impl Into<BulkString>) -> PreparedCommand<Self, FtInfoResult>
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
    fn ft_list<R, RR>(&mut self) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
        R: FromValue,
        RR: FromSingleValueArray<R>,
    {
        prepare_command(self, cmd("FT._LIST"))
    }

    /// Perform a [`ft_search`](crate::SearchCommands::ft_search)
    /// or [`ft_aggregate`](crate::SearchCommands::ft_aggregate) command and collects performance information
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `query_type` - SEARCH or AGGREGATE query type
    /// * `limited` - if set, removes details of reader iterator.
    /// * `query` - collection of query parameters (non including the index name)
    /// 
    /// # Note
    /// To reduce the size of the output, use [`nocontent`](FtSearchOptions::nocontent) or [`limit(0,0)`](FtSearchOptions::limit) to reduce results reply 
    /// or `LIMITED` to not reply with details of `reader iterators` inside builtin-unions such as `fuzzy` or `prefix`.
    ///
    /// # Return
    /// An instance of [`FtProfileQueryType`](crate::FtProfileQueryType)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.profile/>](https://redis.io/commands/ft.profile/)
    #[must_use]
    fn ft_profile<I, Q, QQ>(
        &mut self,
        index: I,
        query_type: FtProfileQueryType,
        limited: bool,
        query: QQ,
    ) -> PreparedCommand<Self, FtProfileResult>
    where
        Self: Sized,
        I: Into<BulkString>,
        Q: Into<BulkString>,
        QQ: SingleArgOrCollection<Q>
    {
        prepare_command(
            self,
            cmd("FT.PROFILE")
                .arg(index)
                .arg(query_type)
                .arg_if(limited, "LIMITED")
                .arg("QUERY")
                .arg(query),
        )
    }

    /// Search the index with a textual query, returning either documents or just ids
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `query` - text query to search. Refer to [`Query syntax`](https://redis.io/docs/stack/search/reference/query_syntax) for more details.
    /// * `options` - See [`FtSearchOptions`](crate::FtSearchOptions)
    ///
    /// # Return
    /// An instance of [`FtQueryResult`](crate::FtQueryResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.search/>](https://redis.io/commands/ft.search/)
    #[must_use]
    fn ft_search<I, Q>(
        &mut self,
        index: I,
        query: Q,
        options: FtSearchOptions,
    ) -> PreparedCommand<Self, FtQueryResult>
    where
        Self: Sized,
        I: Into<BulkString>,
        Q: Into<BulkString>,
    {
        prepare_command(
            self,
            cmd("FT.SEARCH")
                .arg(index)
                .arg(query)
                .arg(options),
        ).keep_command_for_result()
    }

    /// Perform spelling correction on a query, returning suggestions for misspelled terms
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `query` - search query. See [`Spellchecking`](https://redis.io/docs/stack/search/reference/spellcheck) for more details.
    /// * `options` - See [`FtSpellCheckOptions`](crate::FtSpellCheckOptions)
    ///
    /// # Return
    /// An instance of [`FtSpellCheckResult`](crate::FtSpellCheckResult)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.spellcheck/>](https://redis.io/commands/ft.spellcheck/)
    #[must_use]
    fn ft_spellcheck<I, Q>(
        &mut self,
        index: I,
        query: Q,
        options: FtSpellCheckOptions,
    ) -> PreparedCommand<Self, FtSpellCheckResult>
    where
        Self: Sized,
        I: Into<BulkString>,
        Q: Into<BulkString>,
    {
        prepare_command(
            self,
            cmd("FT.SPELLCHECK")
                .arg(index)
                .arg(query)
                .arg(options)
        )
    }

    /// Dump the contents of a synonym group
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    ///
    /// # Return
    /// This command returns a list of synonym terms and their synonym group ids.
    ///
    /// # See Also
    /// * [<https://redis.io/commands/ft.syndump/>](https://redis.io/commands/ft.syndump/)
    /// * [`Synonym support`](https://redis.io/docs/stack/search/reference/synonyms/)
    #[must_use]
    fn ft_syndump<I, R>(
        &mut self,
        index: I,
    ) -> PreparedCommand<Self, R>
    where
        Self: Sized,
        I: Into<BulkString>,
        R: FromKeyValueValueArray<String, Vec<String>> 
    {
        prepare_command(self, cmd("FT.SYNDUMP").arg(index)
        )
    }

    /// Update a synonym group
    /// 
    /// Use this command to create or update a synonym group with additional terms. 
    /// The command triggers a scan of all documents.    /// 
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
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
    fn ft_synupdate<T: Into<BulkString>>(
        &mut self,
        index: impl Into<BulkString>,
        synonym_group_id: impl Into<BulkString>,
        skip_initial_scan: bool,
        terms: impl SingleArgOrCollection<T>
    ) -> PreparedCommand<Self, ()>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FT.SYNUPDATE")
            .arg(index)
            .arg(synonym_group_id)
            .arg_if(skip_initial_scan, "SKIPINITIALSCAN")
            .arg(terms)
        )
    }

    /// Return a distinct set of values indexed in a Tag field
    /// 
    /// Use this command if your tag indexes things like cities, categories, and so on.
    ///
    /// # Arguments
    /// * `index` - index name. You must first create the index using [`ft_create`](crate::SearchCommands::ft_create).
    /// * `field_name` - name of a Tag file defined in the schema.
    ///
    /// # Return
    /// A coolection reply of all distinct tags in the tag index.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.tagvals/>](https://redis.io/commands/ft.tagvals/)
    #[must_use]
    fn ft_tagvals<R: FromValue, RR: FromSingleValueArray<R>>(
        &mut self,
        index: impl Into<BulkString>,
        field_name: impl Into<BulkString>,
    ) -> PreparedCommand<Self, RR>
    where
        Self: Sized,
    {
        prepare_command(self, cmd("FT.TAGVALS")
            .arg(index)
            .arg(field_name)
        )
    }
}

/// Field type used to declare an index schema
/// for the [`ft_create`](crate::SearchCommands::ft_create) command
#[derive(Debug)]
pub enum FtFieldType {
    /// Allows full-text search queries against the value in this attribute.
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
    Vector,
}

impl IntoArgs for FtFieldType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            FtFieldType::Text => "TEXT",
            FtFieldType::Tag => "TAG",
            FtFieldType::Numeric => "NUMERIC",
            FtFieldType::Geo => "GEO",
            FtFieldType::Vector => "VECTOR",
        })
    }
}

impl FromValue for FtFieldType {
    fn from_value(value: Value) -> Result<Self> {
        match value.into::<String>()?.as_str() {
            "TEXT" => Ok(FtFieldType::Text),
            "TAG" => Ok(FtFieldType::Tag),
            "NUMERIC" => Ok(FtFieldType::Numeric),
            "GEO" => Ok(FtFieldType::Geo),
            "VECTOR" => Ok(FtFieldType::Vector),
            s => Err(Error::Client(format!("Cannot parse {s} to FtFieldType"))),
        }
    }
}

/// Phonetic algorithm and language used for the [`FtFieldSchema::phonetic`](crate::FtFieldSchema::phonetic) method
///
/// For more information, see [`Phonetic Matching`](https://redis.io/docs/stack/search/reference/phonetic_matching).
pub enum FtPhoneticMatcher {
    /// Double metaphone for English
    DmEn,
    /// Double metaphone for French
    DmFr,
    /// Double metaphone for Portuguese
    DmPt,
    /// Double metaphone for Spanish
    DmEs,
}

impl IntoArgs for FtPhoneticMatcher {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            FtPhoneticMatcher::DmEn => "dm:en",
            FtPhoneticMatcher::DmFr => "dm:fr",
            FtPhoneticMatcher::DmPt => "dm:pt",
            FtPhoneticMatcher::DmEs => "dm:es",
        })
    }
}

/// field schema for the [`ft_create`](crate::SearchCommands::ft_create) command
#[derive(Default)]
pub struct FtFieldSchema {
    command_args: CommandArgs,
}

impl FtFieldSchema {
    /// * For hashes, is a field name within the hash.
    /// * For JSON, the identifier is a JSON Path expression.
    #[must_use]
    pub fn identifier<N: Into<BulkString>>(identifier: N) -> Self {
        Self {
            command_args: CommandArgs::Empty.arg(identifier),
        }
    }

    /// Defines the attribute associated to the identifier.
    ///
    ///  For example, you can use this feature to alias a complex JSONPath
    ///  expression with more memorable (and easier to type) name.
    #[must_use]
    pub fn as_attribute<A: Into<BulkString>>(self, as_attribute: A) -> Self {
        Self {
            command_args: self.command_args.arg("AS").arg(as_attribute),
        }
    }

    /// The field type.
    ///
    /// Mandatory option to be used after `identifier` or `as_attribute`
    #[must_use]
    pub fn field_type(self, field_type: FtFieldType) -> Self {
        Self {
            command_args: self.command_args.arg(field_type),
        }
    }

    /// Numeric, tag (not supported with JSON) or text attributes can have the optional `SORTABLE` argument.
    ///
    /// As the user [`sorts the results by the value of this attribute`](https://redis.io/docs/stack/search/reference/sorting),
    /// the results will be available with very low latency.
    /// (this adds memory overhead so consider not to declare it on large text attributes).
    #[must_use]
    pub fn sortable(self) -> Self {
        Self {
            command_args: self.command_args.arg("SORTABLE"),
        }
    }

    /// By default, SORTABLE applies a normalization to the indexed value (characters set to lowercase, removal of diacritics).
    ///  When using un-normalized form (UNF), you can disable the normalization and keep the original form of the value.
    #[must_use]
    pub fn unf(self) -> Self {
        Self {
            command_args: self.command_args.arg("UNF"),
        }
    }

    /// Text attributes can have the `NOSTEM` argument which will disable stemming when indexing its values.
    /// This may be ideal for things like proper names.
    #[must_use]
    pub fn nostem(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOSTEM"),
        }
    }

    /// Attributes can have the `NOINDEX` option, which means they will not be indexed.
    ///
    /// This is useful in conjunction with `SORTABLE`,
    /// to create attributes whose update using PARTIAL will not cause full reindexing of the document.
    /// If an attribute has NOINDEX and doesn't have SORTABLE, it will just be ignored by the index.
    #[must_use]
    pub fn noindex(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOINDEX"),
        }
    }

    /// Attributes can have the `NOINDEX` option, which means they will not be indexed.
    ///
    /// This is useful in conjunction with `SORTABLE`,
    /// to create attributes whose update using PARTIAL will not cause full reindexing of the document.
    /// If an attribute has NOINDEX and doesn't have SORTABLE, it will just be ignored by the index.
    #[must_use]
    pub fn phonetic(self, matcher: FtPhoneticMatcher) -> Self {
        Self {
            command_args: self.command_args.arg("PHONETIC").arg(matcher),
        }
    }

    /// for `TEXT` attributes, declares the importance of this attribute when calculating result accuracy.
    ///
    /// This is a multiplication factor, and defaults to 1 if not specified.
    #[must_use]
    pub fn weight(self, weight: f64) -> Self {
        Self {
            command_args: self.command_args.arg("WEIGHT").arg(weight),
        }
    }

    /// for `TAG` attributes, indicates how the text contained in the attribute is to be split into individual tags.
    /// The default is `,`. The value must be a single character.
    #[must_use]
    pub fn separator(self, sep: char) -> Self {
        Self {
            command_args: self.command_args.arg("SEPARATOR").arg(sep),
        }
    }

    /// for `TAG` attributes, keeps the original letter cases of the tags.
    /// If not specified, the characters are converted to lowercase.
    #[must_use]
    pub fn case_sensitive(self) -> Self {
        Self {
            command_args: self.command_args.arg("CASESENSITIVE"),
        }
    }

    /// for `TEXT` and `TAG` attributes, keeps a suffix [`trie`](https://en.wikipedia.org/wiki/Trie)
    ///  with all terms which match the suffix.
    ///
    /// It is used to optimize `contains` (foo) and `suffix` (*foo) queries.
    /// Otherwise, a brute-force search on the trie is performed.
    /// If suffix trie exists for some fields, these queries will be disabled for other fields.
    #[must_use]
    pub fn with_suffix_trie(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSUFFIXTRIE"),
        }
    }
}

impl IntoArgs for FtFieldSchema {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Redis Data type of an index defined in [`FtCreateOptions`](crate::FtCreateOptions) struct
#[derive(Debug)]
pub enum FtIndexDataType {
    /// [`hash`](https://redis.io/docs/data-types/hashes/) (default)
    Hash,
    /// [`json`](https://redis.io/docs/stack/json)
    Json,
}

impl IntoArgs for FtIndexDataType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            FtIndexDataType::Hash => "HASH",
            FtIndexDataType::Json => "JSON",
        })
    }
}

impl FromValue for FtIndexDataType {
    fn from_value(value: Value) -> Result<Self> {
        match value.into::<String>()?.as_str() {
            "HASH" => Ok(FtIndexDataType::Hash),
            "JSON" => Ok(FtIndexDataType::Json),
            s => Err(Error::Client(format!(
                "Cannot parse {s} to FtIndexDataType"
            ))),
        }
    }
}

/// Options for the [`ft_create`](crate::SearchCommands::ft_create) command
#[derive(Default)]
pub struct FtCreateOptions {
    command_args: CommandArgs,
}

impl FtCreateOptions {
    /// currently supports HASH (default) and JSON.
    /// To index JSON, you must have the [`RedisJSON`](https://redis.io/docs/stack/json) module installed.
    #[must_use]
    pub fn on(self, data_type: FtIndexDataType) -> Self {
        Self {
            command_args: self.command_args.arg("ON").arg(data_type),
        }
    }

    /// tells the index which keys it should index.
    ///
    /// You can add several prefixes to index.
    /// Because the argument is optional, the default is * (all keys).
    #[must_use]
    pub fn prefix<P: Into<BulkString>, PP: SingleArgOrCollection<P>>(self, prefixes: PP) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("PREFIX")
                .arg(prefixes.num_args())
                .arg(prefixes),
        }
    }

    /// filter expression with the full RediSearch aggregation expression language.
    ///
    /// It is possible to use `@__key` to access the key that was just added/changed.
    /// A field can be used to set field name by passing `FILTER @indexName=="myindexname"`.
    #[must_use]
    pub fn filter<F: Into<BulkString>>(self, filter: F) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER").arg(filter),
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
    pub fn language(self, default_lang: FtLanguage) -> Self {
        Self {
            command_args: self.command_args.arg("LANGUAGE").arg(default_lang),
        }
    }

    /// document attribute set as the document language.
    #[must_use]
    pub fn language_field<L: Into<BulkString>>(self, default_lang: L) -> Self {
        Self {
            command_args: self.command_args.arg("LANGUAGE_FIELD").arg(default_lang),
        }
    }

    /// default score for documents in the index.
    ///
    /// Default score is 1.0.
    #[must_use]
    pub fn score(self, default_score: f64) -> Self {
        Self {
            command_args: self.command_args.arg("SCORE").arg(default_score),
        }
    }

    /// document attribute that you use as the document rank based on the user ranking.
    ///
    /// Ranking must be between 0.0 and 1.0. If not set, the default score is 1.
    #[must_use]
    pub fn score_field<S: Into<BulkString>>(self, score_attribute: S) -> Self {
        Self {
            command_args: self.command_args.arg("SCORE_FIELD").arg(score_attribute),
        }
    }

    /// document attribute that you use as a binary safe payload string to the document
    /// that can be evaluated at query time by a custom scoring function or retrieved to the client.
    #[must_use]
    pub fn payload_field<P: Into<BulkString>>(self, payload_attribute: P) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("PAYLOAD_FIELD")
                .arg(payload_attribute),
        }
    }

    /// forces RediSearch to encode indexes as if there were more than 32 text attributes,
    /// which allows you to add additional attributes (beyond 32) using [`ft_alter`](crate::SearchCommands::ft_alter).
    ///
    /// For efficiency, RediSearch encodes indexes differently if they are created with less than 32 text attributes.
    #[must_use]
    pub fn max_text_fields(self) -> Self {
        Self {
            command_args: self.command_args.arg("MAXTEXTFIELDS"),
        }
    }

    /// does not store term offsets for documents.
    ///
    /// It saves memory, but does not allow exact searches or highlighting.
    /// It implies [`NOHL`](crate::FtCreateOptions::nohl).
    #[must_use]
    pub fn no_offsets(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOOFFSETS"),
        }
    }

    /// creates a lightweight temporary index that expires after a specified period of inactivity.
    ///
    /// * `expiration_sec` - index will expire after this duration in seconds.
    ///
    /// The internal idle timer is reset whenever the index is searched or added to.
    /// Because such indexes are lightweight,
    /// you can create thousands of such indexes without negative performance implications and, therefore,
    /// you should consider using [`SKIPINITIALSCAN`](crate::FtCreateOptions::skip_initial_scan) to avoid costly scanning.
    #[must_use]
    pub fn temporary(self, expiration_sec: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TEMPORARY").arg(expiration_sec),
        }
    }

    /// conserves storage space and memory by disabling highlighting support.
    ///
    /// If set, the corresponding byte offsets for term positions are not stored.
    /// `NOHL` is also implied by [`NOOFFSETS`](crate::FtCreateOptions::no_offsets).
    #[must_use]
    pub fn nohl(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOHL"),
        }
    }

    /// does not store attribute bits for each term.
    ///
    /// It saves memory, but it does not allow filtering by specific attributes.
    #[must_use]
    pub fn nofields(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOFIELDS"),
        }
    }

    /// avoids saving the term frequencies in the index.
    ///
    /// It saves memory, but does not allow sorting based on the frequencies of a given term within the document.
    #[must_use]
    pub fn nofreqs(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOFREQS"),
        }
    }

    /// if set, does not scan and index.
    #[must_use]
    pub fn skip_initial_scan(self) -> Self {
        Self {
            command_args: self.command_args.arg("SKIPINITIALSCAN"),
        }
    }

    /// sets the index with a custom stopword list, to be ignored during indexing and search time.
    ///
    /// # Arguments
    /// * `stop_words` - a list of stopword arguments.
    ///
    /// If not set, [`FT.CREATE`](crate::SearchCommands::ft_create) takes the default list of stopwords.
    /// If `count` is set to 0, the index does not have stopwords.
    #[must_use]
    pub fn stop_words<W, WW>(self, stop_words: WW) -> Self
    where
        W: Into<BulkString>,
        WW: SingleArgOrCollection<W>,
    {
        Self {
            command_args: self
                .command_args
                .arg("STOPWORDS")
                .arg(stop_words.num_args())
                .arg(stop_words),
        }
    }
}

impl IntoArgs for FtCreateOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Options for the [`ft_create`](crate::SearchCommands::ft_aggregate) command
#[derive(Default)]
pub struct FtAggregateOptions {
    command_args: CommandArgs,
}

impl FtAggregateOptions {
    /// if set, does not try to use stemming for query expansion but searches the query terms verbatim.
    ///
    /// Attributes needed for aggregations should be stored as [`SORTABLE`](crate::FtFieldSchema::sortable),
    /// where they are available to the aggregation pipeline with very low latency.
    /// `LOAD` hurts the performance of aggregate queries considerably because every processed record
    /// needs to execute the equivalent of [`HMGET`](crate::HashCommands::hmget) against a Redis key,
    /// which when executed over millions of keys, amounts to high processing times.
    #[must_use]
    pub fn verbatim(self) -> Self {
        Self {
            command_args: self.command_args.arg("VERBATIM"),
        }
    }

    /// loads document attributes from the source document.
    #[must_use]
    pub fn load<A: ArgsOrCollection<FtLoadAttribute>>(self, attributes: A) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("LOAD")
                .arg(attributes.num_args())
                .arg(attributes),
        }
    }

    /// all attributes in a document are loaded.
    #[must_use]
    pub fn load_all(self) -> Self {
        Self {
            command_args: self.command_args.arg("LOAD").arg("*"),
        }
    }

    /// groups the results in the pipeline based on one or more properties.
    ///
    /// Each group should have at least one reducer,
    /// a function that handles the group entries,
    /// either counting them,
    /// or performing multiple aggregate operations (see [`FtReducer`](crate::FtReducer)).
    #[must_use]
    pub fn groupby<P, PP, R>(self, properties: PP, reducers: R) -> Self
    where
        P: Into<BulkString>,
        PP: SingleArgOrCollection<P>,
        R: ArgsOrCollection<FtReducer>,
    {
        Self {
            command_args: self
                .command_args
                .arg("GROUPBY")
                .arg(properties.num_args())
                .arg(properties)
                .arg(reducers),
        }
    }

    /// Sort the pipeline up until the point of SORTBY, using a list of properties.
    ///
    /// `max` is used to optimized sorting, by sorting only for the n-largest elements.
    /// Although it is not connected to [`limit`](FtAggregateOptions::limit), you usually need just `SORTBY … MAX` for common queries.
    #[must_use]
    pub fn sortby<P>(self, properties: P, max: Option<usize>) -> Self
    where
        P: ArgsOrCollection<FtSortBy>,
    {
        Self {
            command_args: self
                .command_args
                .arg("SORTBY")
                .arg(properties.num_args())
                .arg(properties)
                .arg(max.map(|m| ("MAX", m))),
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
    pub fn apply<E, N>(self, expr: E, name: N) -> Self
    where
        E: Into<BulkString>,
        N: Into<BulkString>,
    {
        Self {
            command_args: self.command_args.arg("APPLY").arg(expr).arg("AS").arg(name),
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
    pub fn limit(self, offset: usize, num: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(offset).arg(num),
        }
    }

    /// filters the results using predicate expressions relating to values in each result.
    /// They are applied post query and relate to the current state of the pipeline.
    #[must_use]
    pub fn filter<E, N>(self, expr: E) -> Self
    where
        E: Into<BulkString>,
    {
        Self {
            command_args: self.command_args.arg("FILTER").arg(expr),
        }
    }

    /// Scan part of the results with a quicker alternative than [`limit`](crate::FtAggregateOptions::limit).
    /// See [`Cursor API`](https://redis.io/docs/stack/search/reference/aggregations/#cursor-api) for more details.
    #[must_use]
    pub fn withcursor(self, options: FtWithCursorOptions) -> Self {
        Self {
            command_args: self.command_args.arg("WITHCURSOR").arg(options),
        }
    }

    /// if set, overrides the timeout parameter of the module.
    #[must_use]
    pub fn timeout(self, milliseconds: u64) -> Self {
        Self {
            command_args: self.command_args.arg("TIMEOUT").arg(milliseconds),
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
    /// such as in field names, for example, @loc. To use `PARAMS`, set [`dialect`](crate::FtAggregateOptions::dialect) to 2 or greater than 2.
    #[must_use]
    pub fn params<N, V, P>(self, params: P) -> Self
    where
        N: Into<BulkString>,
        V: Into<BulkString>,
        P: ArgsOrCollection<(N, V)>,
    {
        Self {
            command_args: self
                .command_args
                .arg("PARAMS")
                .arg(params.num_args())
                .arg(params),
        }
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](crate::SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(self, dialect_version: u64) -> Self {
        Self {
            command_args: self.command_args.arg("DIALECT").arg(dialect_version),
        }
    }
}

impl IntoArgs for FtAggregateOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Attribute for the [`LOAD`](crate::FtAggregateOptions::load) aggregate option
pub struct FtLoadAttribute {
    command_args: CommandArgs,
}

impl FtLoadAttribute {
    #[must_use]
    /// `identifier` is either an attribute name for hashes and JSON or a JSON Path expression for JSON.
    pub fn new<I: Into<BulkString>>(identifier: I) -> Self {
        Self {
            command_args: CommandArgs::Empty.arg(identifier),
        }
    }

    /// `property` is the optional name used in the result.
    ///
    /// If it is not provided, the identifier is used.
    /// This should be avoided.
    #[must_use]
    pub fn property<P: Into<BulkString>>(property: P) -> Self {
        Self {
            command_args: CommandArgs::Empty.arg("AS").arg(property),
        }
    }
}

impl IntoArgs for FtLoadAttribute {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }

    fn num_args(&self) -> usize {
        self.command_args.len()
    }
}

/// Reducer for the [`groupby`](crate::FtAggregateOptions::groupby) aggregate option
pub struct FtReducer {
    command_args: CommandArgs,
}

impl FtReducer {
    #[must_use]
    /// Count the number of records in each group
    pub fn count() -> FtReducer {
        Self {
            command_args: CommandArgs::Empty.arg("REDUCE").arg("COUNT").arg(0),
        }
    }

    /// Count the number of distinct values for property.
    ///
    /// # Note
    /// The reducer creates a hash-set per group, and hashes each record.
    /// This can be memory heavy if the groups are big.
    pub fn count_distinct<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("COUNT_DISTINCT")
                .arg(1)
                .arg(property),
        }
    }

    /// Same as [`count_distinct`](crate::FtReducer::count_distinct) - but provide an approximation instead of an exact count,
    /// at the expense of less memory and CPU in big groups.
    ///
    /// # Note
    /// The reducer uses [`HyperLogLog`](https://en.wikipedia.org/wiki/HyperLogLog) counters per group,
    /// at ~3% error rate, and 1024 Bytes of constant space allocation per group.
    /// This means it is ideal for few huge groups and not ideal for many small groups.
    /// In the former case, it can be an order of magnitude faster and consume much less memory
    /// than [`count_distinct`](crate::FtReducer::count_distinct),
    /// but again, it does not fit every user case.
    pub fn count_distinctish<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("COUNT_DISTINCTISH")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the sum of all numeric values of a given property in a group.
    ///
    /// Non numeric values if the group are counted as 0.
    pub fn sum<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("SUM")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the minimal value of a property, whether it is a string, number or NULL.
    pub fn min<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("MIN")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the maximal value of a property, whether it is a string, number or NULL.
    pub fn max<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("MAX")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the average value of a numeric property.
    ///
    /// This is equivalent to reducing by sum and count,
    /// and later on applying the ratio of them as an APPLY step.
    pub fn avg<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("AVG")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the [`standard deviation`](https://en.wikipedia.org/wiki/Standard_deviation)
    /// of a numeric property in the group.
    pub fn stddev<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("STDDEV")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the value of a numeric property at a given quantile of the results.
    ///
    /// Quantile is expressed as a number between 0 and 1.
    /// For example, the median can be expressed as the quantile at 0.5, e.g. REDUCE QUANTILE 2 @foo 0.5 AS median .
    /// If multiple quantiles are required, just repeat the QUANTILE reducer for each quantile.
    /// e.g. REDUCE QUANTILE 2 @foo 0.5 AS median REDUCE QUANTILE 2 @foo 0.99 AS p99
    pub fn quantile<P: Into<BulkString>>(property: P, quantile: f64) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("QUANTILE")
                .arg(2)
                .arg(property)
                .arg(quantile),
        }
    }

    /// Merge all `distinct` values of a given property into a single array.
    pub fn tolist<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("TOLIST")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    ///
    /// If no BY is specified, we return the first value we encounter in the group.
    /// If you with to get the top or bottom value in the group sorted by the same value,
    /// you are better off using the MIN/MAX reducers,
    /// but the same effect will be achieved by doing REDUCE FIRST_VALUE 4 @foo BY @foo DESC.
    pub fn first_value<P: Into<BulkString>>(property: P) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("FIRST_VALUE")
                .arg(1)
                .arg(property),
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    pub fn first_value_by<P: Into<BulkString>, BP: Into<BulkString>>(
        property: P,
        by_property: BP,
    ) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("FIRST_VALUE")
                .arg(2)
                .arg(property)
                .arg(by_property),
        }
    }

    /// Return the first or top value of a given property in the group, optionally by comparing that or another property.
    pub fn first_value_by_order<P: Into<BulkString>, BP: Into<BulkString>>(
        property: P,
        by_property: BP,
        order: SortOrder,
    ) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("FIRST_VALUE")
                .arg(3)
                .arg(property)
                .arg(by_property)
                .arg(order),
        }
    }

    /// Perform a reservoir sampling of the group elements with a given size,
    ///  and return an array of the sampled items with an even distribution.
    pub fn random_sample<P: Into<BulkString>, BP: Into<BulkString>>(
        property: P,
        sample_size: usize,
    ) -> FtReducer {
        Self {
            command_args: CommandArgs::Empty
                .arg("REDUCE")
                .arg("RANDOM_SAMPLE")
                .arg(2)
                .arg(property)
                .arg(sample_size),
        }
    }

    /// The reducers can have their own property names using the AS {name} optional argument.
    ///
    /// If a name is not given, the resulting name will be
    /// the name of the reduce function and the group properties.
    /// For example, if a name is not given to COUNT_DISTINCT by property @foo,
    /// the resulting name will be count_distinct(@foo).
    pub fn as_name<N: Into<BulkString>>(self, name: N) -> Self {
        Self {
            command_args: self.command_args.arg("AS").arg(name),
        }
    }
}

impl IntoArgs for FtReducer {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// option for the [`sortby`](crate::FtAggregateOptions::sortby) aggregate option
pub struct FtSortBy {
    command_args: CommandArgs,
}

impl FtSortBy {
    /// sort by property
    pub fn property<P: Into<BulkString>>(property: P) -> FtSortBy {
        Self {
            command_args: CommandArgs::Empty.arg(property),
        }
    }

    /// ascending
    pub fn asc(self) -> FtSortBy {
        Self {
            command_args: self.command_args.arg("ASC"),
        }
    }

    /// ascending
    pub fn desc(self) -> FtSortBy {
        Self {
            command_args: self.command_args.arg("DESC"),
        }
    }
}

impl IntoArgs for FtSortBy {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }

    fn num_args(&self) -> usize {
        self.command_args.len()
    }
}

/// options for the [`withcursor`](crate::FtAggregateOptions::withcursor) aggregate option
#[derive(Default)]
pub struct FtWithCursorOptions {
    command_args: CommandArgs,
}

impl FtWithCursorOptions {
    /// Control how many rows are read per each cursor fetch.
    pub fn count(self, read_size: usize) -> FtWithCursorOptions {
        Self {
            command_args: self.command_args.arg("COUNT").arg(read_size),
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
    pub fn maxidle(self, idle_time_ms: u64) -> FtWithCursorOptions {
        Self {
            command_args: self.command_args.arg("MAXIDLE").arg(idle_time_ms),
        }
    }
}

impl IntoArgs for FtWithCursorOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Result for the [`ft_aggregate`](crate::SearchCommands::ft_aggregate) 
/// & [`ft_search`](crate::SearchCommands::ft_search) commands
#[derive(Debug)]
pub struct FtQueryResult {
    pub total_results: usize,
    pub results: Vec<FtQueryResultRow>,
    pub cursor_id: Option<u64>,
}

/// A row in a [`FtQueryResult`](crate::FtQueryResult)
#[derive(Debug, Default)]
pub struct FtQueryResultRow {
    /// Will be empty for [`ft_aggregate`](crate::SearchCommands::ft_aggregate) 
    pub document_id: String,
    /// relative internal score of each document. only if [`withscores`](crate::FtSearchOptions::withscores) is set
    pub score: f64,
    /// document payload. only if [`withpayloads`](crate::FtSearchOptions::withpayloads) is set
    pub payload: Vec<u8>,
    /// value of the sorting key. only if [`withsortkeys`](crate::FtSearchOptions::withsortkeys) is set
    pub sortkey: String,
    /// collection of attribute/value pairs.
    pub values: Vec<(String, String)>,
}

impl FtQueryResult {
    fn from_value(value: Value, is_search: bool, nocontent: bool, withscores: bool, withpayloads: bool, withsortkeys: bool) -> Result<Self> {
        let values: Vec<Value> = value.into()?;

        match &values[0] {
            // regular results
            Value::Integer(_) => {
                let mut iter = values.into_iter();

                let total_results = if let Some(total_results) = iter.next() {
                    total_results.into()?
                } else {
                    return Err(Error::Client("Cannot parse FtQueryResult from result".to_owned()));
                };

                if is_search {
                    // search
                    let mut results = Vec::<FtQueryResultRow>::new();

                    while let Some(value) = iter.next() {
                        let document_id = value.into()?;

                        let score = if withscores {
                            if let Some(score) = iter.next() {
                                score.into()?
                            } else {
                                return Err(Error::Client("Cannot parse FtQueryResult from result".to_owned()));
                            }
                        } else {
                            Default::default()
                        };

                        let payload = if withpayloads {
                            if let Some(Value::BulkString(BulkString::Binary(payload))) = iter.next() {
                                payload
                            } else {
                                return Err(Error::Client("Cannot parse FtQueryResult from result".to_owned()));
                            }
                        } else {
                            Default::default()
                        };

                        let sortkey = if withsortkeys {
                            if let Some(sortkey) = iter.next() {
                                sortkey.into()?
                            } else {
                                return Err(Error::Client("Cannot parse FtQueryResult from result".to_owned()));
                            }
                        } else {
                            Default::default()
                        };

                        let values = if !nocontent {
                            if let Some(sortkey) = iter.next() {
                                sortkey.into()?
                            } else {
                                return Err(Error::Client("Cannot parse FtQueryResult from result".to_owned()));
                            }
                        } else {
                            Default::default()
                        };

                        results.push(FtQueryResultRow {
                            document_id,
                            score,
                            payload,
                            sortkey,
                            values
                        });
                    }

                    Ok(Self {
                        total_results,
                        results,
                        cursor_id: None,
                    }) 
                } else {
                    // regular aggregate
                    Ok(Self {
                        total_results,
                        results: iter.map(|v| Ok(FtQueryResultRow {
                            document_id: "".to_owned(),
                            values: v.into()?,
                            ..Default::default()
                        })).collect::<Result<Vec<FtQueryResultRow>>>()?,
                        cursor_id: None,
                    }) 
                }
            }
            // aggregate WITHCURSOR
            Value::Array(_) => {
                let mut iter = values.into_iter();

                match (iter.next(), iter.next(), iter.next()) {
                    (Some(results), Some(cursor_id), None) => {
                        let results: Vec<Value> = results.into()?;

                        let mut iter = results.into_iter();

                        let total_results = if let Some(total_results) = iter.next() {
                            total_results.into()?
                        } else {
                            return Err(Error::Client("Cannot parse FtQueryResult from result".to_owned()));
                        };

                        Ok(Self {
                            total_results,
                            results: iter.map(|v| Ok(FtQueryResultRow {
                                document_id: "".to_owned(),
                                values: v.into()?,
                                ..Default::default()
                            })).collect::<Result<Vec<FtQueryResultRow>>>()?,
                            cursor_id: Some(cursor_id.into()?),
                        }) 
                    }
                    _ => Err(Error::Client(
                        "Cannot parse FtQueryResult from result".to_owned(),
                    )),
                }
            }
            _ => Err(Error::Client(
                "Cannot parse FtQueryResult from result".to_owned(),
            )),
        }
    }  
}

impl FromValue for FtQueryResult {
    fn from_value(value: Value) -> Result<Self> {
        log::debug!("value: {:?}", value);
        let is_search = if let Value::Array(Array::Vec(ref values)) = &value {
            log::debug!("&values[0..2]: {:?}", &values[0..2]);
            matches!(&values[0..2], [Value::Integer(_total_results), Value::BulkString(BulkString::Binary(_doc_id))])
        } else {
            false
        };

        Self::from_value(value, is_search, false, false, false, false)
    }

    fn from_value_with_command(value: Value, command: &Command) -> Result<Self> {
        let is_search = command.name == "FT.SEARCH";
        let mut nocontent = false;
        let mut withscores = false;
        let mut withpayloads = false;
        let mut withsortkeys = false;

        for arg in command.args.into_iter() {
            match arg {
                BulkString::Str("NOCONTENT") => nocontent = true,
                BulkString::Str("WITHSCORES") => withscores = true,
                BulkString::Str("WITHPAYLOADS") => withpayloads = true,
                BulkString::Str("WITHSORTKEYS") => withsortkeys = true,
                _ => ()
            }
        }

        Self::from_value(value, is_search, nocontent, withscores, withpayloads, withsortkeys)
    }

}

/// Result for the [`ft_info`](crate::SearchCommands::ft_info) command
#[derive(Debug)]
pub struct FtInfoResult {
    /// Name of the index
    pub index_name: String,
    /// index [`creation`](crate::SearchCommands::ft_create) options without paramater
    pub index_options: Vec<String>,
    /// index [`creation`](crate::SearchCommands::ft_create) options with a paramater
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
    pub gc_stats: FtGcStats,
    pub cursor_stats: FtCursorStats,
    /// if a custom stopword list is used.
    pub stopwords_list: Vec<String>,
}

impl FromValue for FtInfoResult {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            index_name: values.remove_with_result("index_name")?.into()?,
            index_options: values.remove_with_result("index_options")?.into()?,
            index_definition: values.remove_with_result("index_definition")?.into()?,
            attributes: values.remove_with_result("attributes")?.into()?,
            num_docs: values.remove_or_default("num_docs").into()?,
            max_doc_id: values.remove_or_default("max_doc_id").into()?,
            num_terms: values.remove_or_default("num_terms").into()?,
            num_records: values.remove_or_default("num_records").into()?,
            inverted_sz_mb: values.remove_or_default("inverted_sz_mb").into()?,
            vector_index_sz_mb: values.remove_or_default("vector_index_sz_mb").into()?,
            total_inverted_index_blocks: values
                .remove_or_default("total_inverted_index_blocks")
                .into()?,
            offset_vectors_sz_mb: values.remove_or_default("offset_vectors_sz_mb").into()?,
            doc_table_size_mb: values.remove_or_default("doc_table_size_mb").into()?,
            sortable_values_size_mb: values.remove_or_default("sortable_values_size_mb").into()?,
            key_table_size_mb: values.remove_or_default("key_table_size_mb").into()?,
            records_per_doc_avg: values.remove_or_default("records_per_doc_avg").into()?,
            bytes_per_record_avg: values.remove_or_default("bytes_per_record_avg").into()?,
            offsets_per_term_avg: values.remove_or_default("offsets_per_term_avg").into()?,
            offset_bits_per_record_avg: values
                .remove_or_default("offset_bits_per_record_avg")
                .into()?,
            hash_indexing_failures: values.remove_or_default("hash_indexing_failures").into()?,
            total_indexing_time: values.remove_or_default("total_indexing_time").into()?,
            indexing: values.remove_or_default("indexing").into()?,
            percent_indexed: values.remove_or_default("percent_indexed").into()?,
            number_of_uses: values.remove_or_default("number_of_uses").into()?,
            gc_stats: values.remove_or_default("gc_stats").into()?,
            cursor_stats: values.remove_or_default("cursor_stats").into()?,
            stopwords_list: values.remove_or_default("stopwords_list").into()?,
        })
    }
}

/// Index attribute info
#[derive(Debug)]
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
}

impl FromValue for FtIndexAttribute {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: Vec<Value> = value.into()?;
        let mut sortable = false;

        if let Some(Value::SimpleString(s)) = values.last() {
            if s == "SORTABLE" {
                values.pop();
                sortable = true;
            }
        }

        let mut values: HashMap<String, Value> = values
            .into_value_iter()
            .collect::<Result<HashMap<String, Value>>>()?;

        Ok(Self {
            identifier: values.remove_with_result("identifier")?.into()?,
            attribute: values.remove_with_result("attribute")?.into()?,
            field_type: values.remove_with_result("type")?.into()?,
            weight: values.remove_or_default("WEIGHT").into()?,
            sortable,
        })
    }
}

/// Garbage collector stats for the [`ft_info`](crate::SearchCommands::ft_info) command
#[derive(Debug)]
pub struct FtGcStats {
    pub bytes_collected: usize,
    pub total_ms_run: usize,
    pub total_cycles: usize,
    pub average_cycle_time_ms: f64,
    pub last_run_time_ms: usize,
    pub gc_numeric_trees_missed: usize,
    pub gc_blocks_denied: usize,
}

impl FromValue for FtGcStats {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            bytes_collected: values.remove_or_default("bytes_collected").into()?,
            total_ms_run: values.remove_or_default("total_ms_run").into()?,
            total_cycles: values.remove_or_default("total_cycles").into()?,
            average_cycle_time_ms: values.remove_or_default("average_cycle_time_ms").into()?,
            last_run_time_ms: values.remove_or_default("last_run_time_ms").into()?,
            gc_numeric_trees_missed: values.remove_or_default("gc_numeric_trees_missed").into()?,
            gc_blocks_denied: values.remove_or_default("gc_blocks_denied").into()?,
        })
    }
}

/// Cursor stats for the [`ft_info`](crate::SearchCommands::ft_info) command
#[derive(Debug)]
pub struct FtCursorStats {
    pub global_idle: usize,
    pub global_total: usize,
    pub index_capacity: usize,
    pub index_total: usize,
}

impl FromValue for FtCursorStats {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            global_idle: values.remove_or_default("global_idle").into()?,
            global_total: values.remove_or_default("global_total").into()?,
            index_capacity: values.remove_or_default("index_capacity").into()?,
            index_total: values.remove_or_default("index_total").into()?,
        })
    }
}

/// Index definitin for the [`ft_info`](crate::SearchCommands::ft_info) command
#[derive(Debug)]
pub struct FtIndexDefinition {
    pub key_type: FtIndexDataType,
    pub prefixes: Vec<String>,
    pub filter: String,
    pub default_language: String,
    pub language_field: String,
    pub default_score: f64,
    pub score_field: String,
    pub payload_field: String,
}

impl FromValue for FtIndexDefinition {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            key_type: values.remove_with_result("key_type")?.into()?,
            prefixes: values.remove_with_result("prefixes")?.into()?,
            default_score: values.remove_or_default("default_score").into()?,
            score_field: values.remove_or_default("score_field").into()?,
            filter: values.remove_or_default("filter").into()?,
            default_language: values.remove_or_default("default_language").into()?,
            language_field: values.remove_or_default("language_field").into()?,
            payload_field: values.remove_or_default("payload_field").into()?,
        })
    }
}

/// Type of query for the [`ft_profile`](crate::SearchCommands::ft_profile) command
pub enum FtProfileQueryType {
    /// [`ft_search`](crate::SearchCommands::ft_search) query type
    Search,
    /// [`ft_aggregate`](crate::SearchCommands::ft_aggregate) query type
    Aggregate,
}

impl IntoArgs for FtProfileQueryType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            FtProfileQueryType::Search => "SEARCH",
            FtProfileQueryType::Aggregate => "AGGREGATE",
        })
    }
}

/// Result for the [`ft_profile`](crate::SearchCommands::ft_profile) command.
#[derive(Debug)]
pub struct FtProfileResult {
    pub results: FtQueryResult,
    pub profile_details: FtProfileDetails,
}

impl FromValue for FtProfileResult {
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut iter = values.into_iter();

        let (Some(results), Some(profile_details), None) = (iter.next(), iter.next(), iter.next()) else {
            return Err(Error::Client("Cannot parse FtProfileResult".to_owned()));
        };

        Ok(Self {
            results: results.into()?,
            profile_details: profile_details.into()?,
        })
    }
}

/// Details of a [`ft_profile`](crate::SearchCommands::ft_profile) command.
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
    pub result_processors_profile: Vec<FtResultProcessorsProfile>    
}

impl FromValue for FtProfileDetails {
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut iter = values.into_iter();

        let (Some(total_profile_time), Some(parsing_time), Some(pipeline_creation_time), Some(iterators_profile), Some(result_processors_profile)) 
            = (iter.next(), iter.next(), iter.next(), iter.next(), iter.next()) else {
            return Err(Error::Client("Cannot parse FtProfileResult".to_owned()));
        };

        let values: Vec<Value> = result_processors_profile.into()?;
        let mut iter = values.into_iter();
 
        match iter.next() {
            Some(Value::SimpleString(s)) if s == "Result processors profile" => (),
            _ => return Err(Error::Client("Cannot parse FtProfileResult".to_owned())),
        }

        let result_processors_profile = Value::Array(Array::Vec(iter.collect()));

        Ok(Self {
            total_profile_time: total_profile_time.into::<HashMap<String, Value>>()?.remove_or_default("Total profile time").into()?,
            parsing_time: parsing_time.into::<HashMap<String, Value>>()?.remove_or_default("Parsing time").into()?,
            pipeline_creation_time: pipeline_creation_time.into::<HashMap<String, Value>>()?.remove_or_default("Pipeline creation time").into()?,
            iterators_profile: iterators_profile.into::<HashMap<String, Value>>()?.remove_or_default("Iterators profile").into()?,
            result_processors_profile: result_processors_profile.into()?,
        })
    }
}

/// Result processors profile for the [`ft_profile`](crate::SearchCommands::ft_profile) command.
#[derive(Debug)]
pub struct FtResultProcessorsProfile {
    pub _type: String,
    pub time: f64,
    pub counter: usize,    
}

impl FromValue for FtResultProcessorsProfile {
    fn from_value(value: Value) -> Result<Self> {
        let mut values: HashMap<String, Value> = value.into()?;

        Ok(Self {
            _type: values.remove_or_default("Type").into()?,
            time: values.remove_or_default("Time").into()?,
            counter: values.remove_or_default("Counter").into()?,
        })
    }
}

/// Options for the [`ft_search`](crate::SearchCommands::ft_search) command.
#[derive(Default)]
pub struct FtSearchOptions {
    command_args: CommandArgs,
}

impl FtSearchOptions {
    /// returns the document ids and not the content. 
    /// This is useful if RediSearch is only an index on an external document collection.
    #[must_use]
    pub fn nocontent(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOCONTENT"),
        }
    }

    /// does not try to use stemming for query expansion but searches the query terms verbatim.
    #[must_use]
    pub fn verbatim(self) -> Self {
        Self {
            command_args: self.command_args.arg("VERBATIM"),
        }
    }

    /// also returns the relative internal score of each document. 
    /// 
    /// This can be used to merge results from multiple instances.
    #[must_use]
    pub fn withscores(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSCORES"),
        }
    }

    /// retrieves optional document payloads. 
    /// 
    /// See [`ft_create`](crate::SearchCommands::ft_create)
    /// The payloads follow the document id and, if [`withscores`](crate::FtSearchOptions::withscores) is set, the scores.
    #[must_use]
    pub fn withpayloads(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHPAYLOADS"),
        }
    }

    /// returns the value of the sorting key, right after the id and score and/or payload, if requested. 
    /// 
    /// This is usually not needed, and exists for distributed search coordination purposes. 
    /// This option is relevant only if used in conjunction with [`sortby`](crate::FtSearchOptions::sortby).
    #[must_use]
    pub fn withsortkeys(self) -> Self {
        Self {
            command_args: self.command_args.arg("WITHSORTKEYS"),
        }
    }

    /// limits results to those having numeric values ranging between min and max, 
    /// if numeric_field is defined as a numeric field in [`ft_create`](crate::SearchCommands::ft_create). 
    /// 
    /// `min` and `max` follow [`zrange`](crate::SortedSetCommands::zrange) syntax, and can be `-inf`, `+inf`, 
    /// and use `(` for exclusive ranges. Multiple numeric filters for different attributes are supported in one query.
    #[must_use]
    pub fn filter(self, numeric_field: impl Into<BulkString>, min: impl Into<BulkString>, max: impl Into<BulkString>) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER").arg(numeric_field).arg(min).arg(max),
        }
    }

    /// filter the results to a given `radius` from `lon` and `lat`. 
    /// 
    /// `radius` is given as a number and units. 
    /// See [`geosearch`](crate::GeoCommands::geosearch) for more details.
    #[must_use]
    pub fn geo_filter(self, geo_field: impl Into<BulkString>, lon: f64, lat: f64, radius: f64, unit: GeoUnit) -> Self {
        Self {
            command_args: self.command_args.arg("GEOFILTER").arg(geo_field).arg(lon).arg(lat).arg(radius).arg(unit),
        }
    }

    /// limits the result to a given set of keys specified in the list. 
    /// 
    /// Non-existent keys are ignored, unless all the keys are non-existent.
    #[must_use]
    pub fn inkeys<A>(self, keys: impl SingleArgOrCollection<A>) -> Self
    where 
        A: Into<BulkString>
    {
        Self {
            command_args: self.command_args.arg("INKEYS").arg(keys.num_args()).arg(keys),
        }
    }

    /// filters the results to those appearing only in specific attributes of the document, like `title` or `URL`. 
    #[must_use]
    pub fn infields<A>(self, attributes: impl SingleArgOrCollection<A>) -> Self
    where 
        A: Into<BulkString>
    {
        Self {
            command_args: self.command_args.arg("INFIELDS").arg(attributes.num_args()).arg(attributes),
        }
    }

    /// limits the attributes returned from the document.
    /// 
    /// If attributes is empty, it acts like [`nocontent`](crate::FtSearchOptions::nocontent). 
    #[must_use]
    pub fn _return(self, attributes: impl ArgsOrCollection<FtSearchReturnAttribute>) -> Self
    {
        Self {
            command_args: self.command_args.arg("RETURN").arg(attributes.num_args()).arg(attributes),
        }
    }

    /// returns only the sections of the attribute that contain the matched text. 
    /// 
    /// See [`Highlighting`](https://redis.io/docs/stack/search/reference/highlight) for more information.
    #[must_use]
    pub fn summarize(self, options: FtSearchSummarizeOptions) -> Self
    {
        Self {
            command_args: self.command_args.arg("SUMMARIZE").arg(options),
        }
    }

    /// formats occurrences of matched text.
    /// 
    /// See [`Highlighting`](https://redis.io/docs/stack/search/reference/highlight) for more information.
    #[must_use]
    pub fn highlight(self, options: FtSearchHighlightOptions) -> Self
    {
        Self {
            command_args: self.command_args.arg("HIGHLIGHT").arg(options),
        }
    }

    /// allows a maximum of N intervening number of unmatched offsets between phrase terms. 
    /// 
    /// In other words, the slop for exact phrases is 0.
    #[must_use]
    pub fn slop(self, slop: usize) -> Self
    {
        Self {
            command_args: self.command_args.arg("SLOP").arg(slop),
        }
    }

    /// puts the query terms in the same order in the document as in the query, 
    /// regardless of the offsets between them. 
    /// 
    /// Typically used in conjunction with [`slop`](crate::FtSearchOptions::slop).
    #[must_use]
    pub fn inorder(self) -> Self
    {
        Self {
            command_args: self.command_args.arg("INORDER"),
        }
    }

    /// use a stemmer for the supplied language during search for query expansion. 
    /// 
    /// If querying documents in Chinese, set to chinese to properly tokenize the query terms. 
    /// Defaults to English. 
    /// If an unsupported language is sent, the command returns an error. 
    /// See FT.CREATE for the list of languages.
    #[must_use]
    pub fn language(self, language: FtLanguage) -> Self
    {
        Self {
            command_args: self.command_args.arg("LANGUAGE").arg(language),
        }
    }

    /// uses a custom query `expander` instead of the stemmer. 
    /// 
    /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn expander(self, expander: impl Into<BulkString>) -> Self
    {
        Self {
            command_args: self.command_args.arg("EXPANDER").arg(expander),
        }
    }

    /// uses a custom scoring function you define. 
    /// 
    /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn scorer(self, scorer: impl Into<BulkString>) -> Self
    {
        Self {
            command_args: self.command_args.arg("SCORER").arg(scorer),
        }
    }

    /// returns a textual description of how the scores were calculated. 
    /// 
    /// Using this options requires the [`withscores`](crate::FtSearchOptions::withscores) option.
    #[must_use]
    pub fn explainscore(self) -> Self
    {
        Self {
            command_args: self.command_args.arg("EXPLAINSCORE"),
        }
    }

    /// adds an arbitrary, binary safe `payload` that is exposed to custom scoring functions.
    /// 
     /// See [`Extensions`](https://redis.io/docs/stack/search/reference/extensions).
    #[must_use]
    pub fn payload(self, payload: impl Into<BulkString>) -> Self
    {
        Self {
            command_args: self.command_args.arg("PAYLOAD").arg(payload),
        }
    }

    /// orders the results by the value of this attribute. 
    /// 
    /// This applies to both text and numeric attributes. 
    /// Attributes needed for `sortby` should be declared as [`SORTABLE`](crate::FtFieldSchema::sortable) in the index, 
    /// in order to be available with very low latency. Note that this adds memory overhead.
    #[must_use]
    pub fn sortby(self, attribute: impl Into<BulkString>, order: SortOrder) -> Self
    {
        Self {
            command_args: self.command_args.arg("SORTBY").arg(attribute).arg(order),
        }
    }

    /// limits the results to the offset and number of results given. 
    /// 
    /// Note that the offset is zero-indexed. The default is `0 10`, which returns 10 items starting from the first result. 
    /// You can use `LIMIT 0 0` to count the number of documents in the result set without actually returning them.
    #[must_use]
    pub fn limit(self, first: usize, num: usize) -> Self
    {
        Self {
            command_args: self.command_args.arg("LIMIT").arg(first).arg(num),
        }
    }

    /// overrides the timeout parameter of the module.
    #[must_use]
    pub fn timeout(self, milliseconds: u64) -> Self
    {
        Self {
            command_args: self.command_args.arg("TIMEOUT").arg(milliseconds),
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
    /// such as in field names, for example, @loc. To use `PARAMS`, set [`dialect`](crate::FtSearchOptions::dialect) to 2 or greater than 2.
    #[must_use]
    pub fn params<N, V, P>(self, params: P) -> Self
    where
        N: Into<BulkString>,
        V: Into<BulkString>,
        P: ArgsOrCollection<(N, V)>,
    {
        Self {
            command_args: self.command_args.arg("PARAMS").arg(params.num_args()).arg(params),
        }
    }

    /// selects the dialect version under which to execute the query.
    ///
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](crate::SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(self, dialect_version: u64) -> Self {
        Self {
            command_args: self.command_args.arg("DIALECT").arg(dialect_version),
        }
    }
}

impl IntoArgs for FtSearchOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// attribute for the [`search`](crate::SearchCommands::ft_search) option [`return`](crate::FtSearchOptions::_return)
pub struct FtSearchReturnAttribute {
    command_args: CommandArgs,
}

impl FtSearchReturnAttribute {
    /// `identifier`is either an attribute name (for hashes and JSON) or a JSON Path expression (for JSON).
    #[must_use]
    pub fn identifier(identifier: impl Into<BulkString>) -> Self {
        Self {
            command_args: CommandArgs::Empty.arg(identifier),
        }
    }

    /// `property`is an optional name used in the result. 
    /// 
    /// If not provided, the `identifier` is used in the result.
    #[must_use]
    pub fn as_property(self, property: impl Into<BulkString>) -> Self {
        Self {
            command_args: self.command_args.arg("AS").arg(property),
        }
    }
}

impl IntoArgs for FtSearchReturnAttribute {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// sub-options for the [`search`](crate::SearchCommands::ft_search) option [`summarize`](crate::FtSearchOptions::summarize)
#[derive(Default)]
pub struct FtSearchSummarizeOptions {
    command_args: CommandArgs,
}

impl FtSearchSummarizeOptions {
    /// If present, must be the first argument.
    /// Each field present is summarized. 
    /// If no `FIELDS` directive is passed, then all fields returned are summarized.
    #[must_use]
    pub fn fields<F: Into<BulkString>>(self, fields: impl SingleArgOrCollection<F>) -> Self {
        Self {
            command_args: self.command_args.arg("FIELDS").arg(fields.num_args()).arg(fields),
        }
    }

    /// How many fragments should be returned. If not specified, a default of 3 is used.
    #[must_use]
    pub fn frags(self, num_frags: usize) -> Self {
        Self {
            command_args: self.command_args.arg("FRAGS").arg(num_frags),
        }
    }

    /// The number of context words each fragment should contain. 
    /// 
    /// Context words surround the found term. 
    /// A higher value will return a larger block of text. 
    /// If not specified, the default value is 20.
    #[must_use]
    pub fn len(self, frag_len: usize) -> Self {
        Self {
            command_args: self.command_args.arg("LEN").arg(frag_len),
        }
    }

    /// The string used to divide between individual summary snippets. 
    /// 
    /// The default is `...` which is common among search engines; 
    /// but you may override this with any other string if you desire to programmatically divide them later on. 
    /// You may use a newline sequence, as newlines are stripped from the result body anyway 
    /// (thus, it will not be conflated with an embedded newline in the text)
    #[must_use]
    pub fn separator(self, separator: impl Into<BulkString>) -> Self {
        Self {
            command_args: self.command_args.arg("SEPARATOR").arg(separator),
        }
    }
}

impl IntoArgs for FtSearchSummarizeOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// sub-options for the [`search`](crate::SearchCommands::ft_search) option [`summarize`](crate::FtSearchOptions::highlight)
#[derive(Default)]
pub struct FtSearchHighlightOptions {
    command_args: CommandArgs,
}

impl FtSearchHighlightOptions {
    /// If present, must be the first argument.
    /// Each field present is highlighted. 
    /// If no `FIELDS` directive is passed, then all fields returned are highlighted.
    #[must_use]
    pub fn fields<F: Into<BulkString>>(self, fields: impl SingleArgOrCollection<F>) -> Self {
        Self {
            command_args: self.command_args.arg("FIELDS").arg(fields.num_args()).arg(fields),
        }
    }

    /// * `open_tag` - prepended to each term match
    /// * `close_tag` - appended to each term match
    /// If no `TAGS` are specified, a built-in tag value is appended and prepended.
    #[must_use]
    pub fn tags(self, open_tag: impl Into<BulkString>, close_tag: impl Into<BulkString>) -> Self {
        Self {
            command_args: self.command_args.arg("TAGS").arg(open_tag).arg(close_tag),
        }
    }
}

impl IntoArgs for FtSearchHighlightOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
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
    Yiddish
}

impl Default for FtLanguage {
    fn default() -> Self {
        Self::English
    }
}

impl IntoArgs for FtLanguage {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(
            match self {
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
            }
        )
    }
}

/// Options for the [`ft_spellcheck`](crate::SearchCommands::ft_spellcheck) command.
#[derive(Default)]
pub struct FtSpellCheckOptions {
    command_args: CommandArgs,
}

impl FtSpellCheckOptions {
    /// maximum Levenshtein distance for spelling suggestions (default: 1, max: 4).
    #[must_use]
    pub fn distance(self, distance: u64) -> Self {
        Self {
            command_args: self.command_args.arg("DISTANCE").arg(distance),
        }
    }

    /// specifies an inclusion (`FtTermType::Include`) or exclusion (`FtTermType::Exclude`) of a custom dictionary named `dictionary`
    /// 
    /// Refer to [`ft_dictadd`](crate::SearchCommands::ft_dictadd), [`ft_dictdel`](crate::SearchCommands::ft_dictdel)
    /// and [`ft_dictdump`](crate::SearchCommands::ft_dictdump) about managing custom dictionaries.
    #[must_use]
    pub fn terms(self, term_type: FtTermType, dictionary: impl Into<BulkString>) -> Self {
        Self {
            command_args: self.command_args.arg("TERMS").arg(term_type).arg(dictionary),
        }
    }

    /// selects the dialect version under which to execute the query. 
    /// 
    /// If not specified, the query will execute under the default dialect version
    /// set during module initial loading or via [`ft_config_set`](crate::SearchCommands::ft_config_set) command.
    #[must_use]
    pub fn dialect(self, dialect_version: u64) -> Self {
        Self {
            command_args: self.command_args.arg("DIALECT").arg(dialect_version),
        }
    }
}

impl IntoArgs for FtSpellCheckOptions {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(self.command_args)
    }
}

/// Term type for the option [`terms`](crate::FtSpellCheckOptions::terms)
pub enum FtTermType {
    Include,
    Exclude,
}

impl IntoArgs for FtTermType {
    fn into_args(self, args: CommandArgs) -> CommandArgs {
        args.arg(match self {
            FtTermType::Include => "INCLUDE",
            FtTermType::Exclude => "EXCLUDE",
        })
    }
}

/// Result for the [`ft_spellcheck`](crate::SearchCommands::ft_spellcheck) command.
pub struct FtSpellCheckResult {
    /// a collection where each element represents a misspelled term from the query + suggestions for this term
    /// 
    /// The misspelled terms are ordered by their order of appearance in the query.
    pub misspelled_terms: Vec<FtMisspelledTerm>
}

impl FromValue for FtSpellCheckResult {
    fn from_value(value: Value) -> Result<Self> {
        Ok(Self {
            misspelled_terms: value.into()?
        })
    }
}

/// Misspelled term + suggestions for the [`ft_spellcheck`](crate::SearchCommands::ft_spellcheck) command.
pub struct FtMisspelledTerm {
    /// Misspelled term
    pub misspelled_term: String,
    /// Suggestion as a tuple composed of
    /// * the score of the suggestion
    /// * the suggestion itself
    pub suggestions: Vec<(f64, String)>,
}

impl FromValue for FtMisspelledTerm {
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut iter = values.into_iter();

        match (iter.next(), iter.next(), iter.next(), iter.next()) {
            (Some(Value::BulkString(BulkString::Binary(term))), Some(misspelled_term), Some(suggestions), None) 
            if term == b"TERM" => {
                Ok(Self {
                    misspelled_term: misspelled_term.into()?,
                    suggestions: suggestions.into()?,
                })
            },
            _ => Err(Error::Client("Cannot parse result to FtMisspelledTerm".to_owned()))
        }
    }
}
