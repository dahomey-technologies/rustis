use crate::{
    prepare_command,
    resp::{
        cmd, ArgsOrCollection, Array, BulkString, CommandArgs, FromKeyValueValueArray,
        FromSingleValueArray, FromValue, IntoArgs, SingleArgOrCollection, Value,
    },
    PreparedCommand, Result, SortOrder, Error,
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
    /// * `options` - See [`FtAggregateOptions`](crate::SearchCommands::FtAggregateOptions)
    ///
    /// # See Also
    /// [<https://redis.io/commands/ft.aggregate/>](https://redis.io/commands/ft.aggregate/)
    /// [`RedisSeach Aggregations`](https://redis.io/docs/stack/search/reference/aggregations/)
    #[must_use]
    fn ft_aggregate<I, Q>(
        &mut self,
        index: I,
        query: Q,
        options: FtAggregateOptions,
    ) -> PreparedCommand<Self, FtAggregateResult>
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
    /// * `alias`- alias to be added to an index
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
    /// * `alias`- alias to be removed
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
    /// * `alias`- alias to be added to an index
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
    /// * `skip_initial_scan`- if set, does not scan and index.
    /// * `attribute`- attribute to add.
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
    /// [<https://redis.io/commands/ft.create/>](https://redis.io/commands/ft.create/)
    /// [`Aggregations`](https://redis.io/docs/stack/search/reference/aggregations/)
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
    fn ft_cursor_read<I>(&mut self, index: I, cursor_id: u64) -> PreparedCommand<Self, FtAggregateResult>
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
        TT: SingleArgOrCollection<T>
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
        TT: SingleArgOrCollection<T>
    {
        prepare_command(self, cmd("FT.DICTDEL").arg(dict).arg(terms))
    }

    /// Dump all terms in the given dictionary
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
        TT: FromSingleValueArray<T>
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
    /// [<https://redis.io/commands/ft.dictadd/>](https://redis.io/commands/ft.dictadd/)
    #[must_use]
    fn ft_dropindex<I>(&mut self, index: I, dd: bool) -> PreparedCommand<Self, usize>
    where
        Self: Sized,
        I: Into<BulkString>,
    {
        prepare_command(self, cmd("FT.DROPINDEX").arg(index).arg_if(dd, "DD"))
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
}

/// Field type used to declare an index schema
/// for the [`ft_create`](crate::SearchCommands::ft_create) command
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

/// Phonetic algorithm and language used for the [FtFieldSchema::phonetic](crate::FtFieldSchema) method
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
    pub fn weight(self, weight: usize) -> Self {
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
    #[must_use]
    pub fn language<L: Into<BulkString>>(self, default_lang: L) -> Self {
        Self {
            command_args: self.command_args.arg("LANGUAGE").arg(default_lang),
        }
    }

    /// document attribute set as the document language.
    ///
    /// Default to English.
    ///
    /// A stemmer is used for the supplied language during indexing.
    /// If an unsupported language is sent, the command returns an error.
    /// The supported languages are Arabic, Basque, Catalan, Danish, Dutch,
    /// English, Finnish, French, German, Greek, Hungarian, Indonesian, Irish,
    ///  Italian, Lithuanian, Nepali, Norwegian, Portuguese, Romanian, Russian,
    /// Spanish, Swedish, Tamil, Turkish, and Chinese.
    ///
    /// When adding Chinese language documents, set `LANGUAGE` chinese for the indexer
    /// to properly tokenize the terms. If you use the default language,
    /// then search terms are extracted based on punctuation characters and whitespace.
    /// The Chinese language tokenizer makes use of a segmentation algorithm (via [`Friso`](https://github.com/lionsoul2014/friso)),
    /// which segments text and checks it against a predefined dictionary.
    /// See [`Stemming`](https://redis.io/docs/stack/search/reference/stemming) for more information.
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
    pub fn score(self, default_score: i64) -> Self {
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
    /// The internal idle timer is reset whenever the index is searched or added to.
    /// Because such indexes are lightweight,
    /// you can create thousands of such indexes without negative performance implications and, therefore,
    /// you should consider using [`SKIPINITIALSCAN`](crate::FtCreateOptions::skip_initial_scan) to avoid costly scanning.
    #[must_use]
    pub fn temporary(self) -> Self {
        Self {
            command_args: self.command_args.arg("TEMPORARY"),
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
    pub fn nofield(self) -> Self {
        Self {
            command_args: self.command_args.arg("NOFIELD"),
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
    /// `count`  is the number of stopwords, followed by a list of stopword arguments exactly the length of {count}.
    ///
    /// If not set, [`FT.CREATE`](crate::SearchCommands::ft_create) takes the default list of stopwords.
    /// If `count` is set to 0, the index does not have stopwords.
    #[must_use]
    pub fn stop_words(self, count: usize) -> Self {
        Self {
            command_args: self.command_args.arg("STOPWORDS").arg(count),
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
    /// or performing multiple aggregate operations (see [`reduce`](crate::SearchCommands::reduce)).
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
    /// Each group should have at least one reducer,
    /// a function that handles the group entries,
    /// either counting them,
    /// or performing multiple aggregate operations (see [`reduce`](crate::SearchCommands::reduce)).
    ///
    /// `MAX` is used to optimized sorting, by sorting only for the n-largest elements.
    /// Although it is not connected to `LIMIT`, you usually need just `SORTBY … MAX` for common queries.
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
    /// such as in field names, for example, @loc. To use `PARAMS`, set [`dialect`](crate::SearchCommands::dialect) to 2 or greater than 2.
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

/// Result for the [`ft_aggregate`](crate::SearchCommands::ft_aggregate) command
pub struct FtAggregateResult {
    pub total_results: usize,
    pub results: Vec<Vec<(String, String)>>,
    pub cursor_id: Option<u64>,
}

impl FromValue for FtAggregateResult {
    fn from_value(value: Value) -> Result<Self> {
        let values: Vec<Value> = value.into()?;

        match &values[0] {
            // regular results
            Value::Integer(_) => {
                let mut iter = values.into_iter();

                let Some(total_results) = iter.next().map(|v| v.into()) else {
                    return Err(Error::Client("Unexpected FT.AGGREGATE result format".to_owned()));
                };
        
                // skip first integer that does not represent a valid value,
                // in accordance to the official documentation
                // see https://redis.io/commands/ft.aggregate/
                let results = iter.collect::<Vec<_>>();
        
                Ok(Self {
                    total_results: total_results?,
                    results: Value::Array(Array::Vec(results)).into()?,
                    cursor_id: None
                })
            },
            // WITHCURSOR results
            Value::Array(_) => {
                let mut iter = values.into_iter();

                match (iter.next(), iter.next(), iter.next()) {
                    (Some(results), Some(cursor_id), None) => {
                        let results: Vec<Value> = results.into()?;

                        let mut iter = results.into_iter();

                        let Some(total_results) = iter.next().map(|v| v.into()) else {
                            return Err(Error::Client("Unexpected FT.AGGREGATE result format".to_owned()));
                        };
                
                        // skip first integer that does not represent a valid value,
                        // in accordance to the official documentation
                        // see https://redis.io/commands/ft.aggregate/
                        let values = iter.collect::<Vec<_>>();
                
                        Ok(Self {
                            total_results: total_results?,
                            results: Value::Array(Array::Vec(values)).into()?,
                            cursor_id: Some(cursor_id.into()?)
                        })

                    },
                    _ => Err(Error::Client("Unexpected FT.AGGREGATE result format".to_owned()))
                }
                
            },
            _ => Err(Error::Client("Unexpected FT.AGGREGATE result format".to_owned()))
        }
    }
}
