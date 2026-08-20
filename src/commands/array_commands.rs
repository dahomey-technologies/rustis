use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{cmd, serialize_flag},
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// A group of Redis commands related to [`Arrays`](https://redis.io/docs/latest/commands/?group=array)
///
/// An array is a sparse, index-addressed collection: every element sits at an
/// explicit zero-based index and the gaps between them cost nothing. That is
/// what separates it from a list — there is no push and pop, only
/// [`arset`](ArrayCommands::arset) at an index you choose, or
/// [`arinsert`](ArrayCommands::arinsert) at the position of a cursor the array
/// carries and you can move with [`arseek`](ArrayCommands::arseek).
///
/// Because of the gaps, two lengths matter and never coincide:
/// [`arlen`](ArrayCommands::arlen) is the highest index plus one, whereas
/// [`arcount`](ArrayCommands::arcount) is how many slots actually hold a value.
///
/// # See Also
/// [Redis Array Commands](https://redis.io/docs/latest/commands/?group=array)
pub trait ArrayCommands<'a>: Sized {
    /// Return the number of non-empty elements in the array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arcount/>](https://redis.io/commands/arcount/)
    #[must_use]
    fn arcount(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARCOUNT").key(key).readonly())
    }

    /// Delete the elements at the given indices.
    ///
    /// # Return
    /// The number of elements that were deleted.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ardel/>](https://redis.io/commands/ardel/)
    #[must_use]
    fn ardel(
        self,
        key: impl Serialize,
        indices: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARDEL").key(key).arg(indices))
    }

    /// Delete the elements in one or more inclusive `(start, end)` ranges.
    ///
    /// # Return
    /// The number of elements that were deleted.
    ///
    /// # See Also
    /// [<https://redis.io/commands/ardelrange/>](https://redis.io/commands/ardelrange/)
    #[must_use]
    fn ardelrange(
        self,
        key: impl Serialize,
        ranges: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARDELRANGE").key(key).arg(ranges))
    }

    /// Get the value at an index.
    ///
    /// # Return
    /// The value, or nil when the slot is empty — so `Option<String>`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arget/>](https://redis.io/commands/arget/)
    #[must_use]
    fn arget<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        index: usize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ARGET").key(key).arg(index).readonly())
    }

    /// Get the values in the inclusive index range `[start, end]`.
    ///
    /// # Return
    /// One entry per index in the range, empty slots included as nil — so
    /// `Vec<Option<String>>`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/argetrange/>](https://redis.io/commands/argetrange/)
    #[must_use]
    fn argetrange<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        start: usize,
        end: usize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ARGETRANGE").key(key).arg(start).arg(end).readonly(),
        )
    }

    /// Search the elements of a range with textual predicates.
    ///
    /// `start` and `end` are indices as strings, so that `-` and `+` can stand
    /// for the first and last index. Giving a `start` greater than `end` walks
    /// the range backwards.
    ///
    /// # Return
    /// The indices of the matching elements — `Vec<usize>` — or index/value
    /// pairs under [`with_values`](ArGrep::with_values), which the server sends
    /// flat and which therefore deserializes as `Vec<(usize, String)>`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/argrep/>](https://redis.io/commands/argrep/)
    #[must_use]
    fn argrep<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        start: impl Serialize,
        end: impl Serialize,
        options: ArGrep,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ARGREP")
                .key(key)
                .arg(start)
                .arg(end)
                .arg(options)
                .readonly(),
        )
    }

    /// Return metadata about the array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arinfo/>](https://redis.io/commands/arinfo/)
    #[must_use]
    fn arinfo(
        self,
        key: impl Serialize,
        options: ArInfoOptions,
    ) -> PreparedCommand<'a, Self, ArrayInfo> {
        prepare_command(self, cmd("ARINFO").key(key).arg(options).readonly())
    }

    /// Insert one or more values at consecutive indices, starting at the
    /// array's insert cursor. The cursor advances by one per value.
    ///
    /// # Return
    /// The last index that was written.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arinsert/>](https://redis.io/commands/arinsert/)
    #[must_use]
    fn arinsert(
        self,
        key: impl Serialize,
        values: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARINSERT").key(key).arg(values))
    }

    /// Return the `count` most recently inserted elements.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arlastitems/>](https://redis.io/commands/arlastitems/)
    #[must_use]
    fn arlastitems<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        count: usize,
        options: ArLastItemsOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ARLASTITEMS")
                .key(key)
                .arg(count)
                .arg(options)
                .readonly(),
        )
    }

    /// Return the length of the array: the highest index in use, plus one.
    ///
    /// Empty slots count towards it; use [`arcount`](ArrayCommands::arcount)
    /// for the number of elements that actually hold a value.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arlen/>](https://redis.io/commands/arlen/)
    #[must_use]
    fn arlen(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARLEN").key(key).readonly())
    }

    /// Get the values at several indices.
    ///
    /// # Return
    /// One entry per requested index, nil where the slot is empty — so
    /// `Vec<Option<String>>`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/armget/>](https://redis.io/commands/armget/)
    #[must_use]
    fn armget<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        indices: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("ARMGET").key(key).arg(indices).readonly())
    }

    /// Set several `(index, value)` pairs at once. The pairs need not be
    /// contiguous nor ordered.
    ///
    /// # Return
    /// The number of slots that were empty before the call.
    ///
    /// # See Also
    /// [<https://redis.io/commands/armset/>](https://redis.io/commands/armset/)
    #[must_use]
    fn armset(
        self,
        key: impl Serialize,
        items: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARMSET").key(key).arg(items))
    }

    /// Return the index [`arinsert`](ArrayCommands::arinsert) would write next.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arnext/>](https://redis.io/commands/arnext/)
    #[must_use]
    fn arnext(self, key: impl Serialize) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARNEXT").key(key).readonly())
    }

    /// Aggregate the non-empty elements of the inclusive range `[start, end]`.
    ///
    /// # Return
    /// Depends on the operation: a bulk string for
    /// [`Sum`](ArOperation::Sum), [`Min`](ArOperation::Min) and
    /// [`Max`](ArOperation::Max), an integer for the bitwise operations,
    /// [`Match`](ArOperation::Match) and [`Used`](ArOperation::Used). Nil when
    /// the range holds nothing to aggregate, so the string-valued operations
    /// want an `Option<String>`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arop/>](https://redis.io/commands/arop/)
    #[must_use]
    fn arop<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        start: usize,
        end: usize,
        operation: ArOperation<'_>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("AROP")
                .key(key)
                .arg(start)
                .arg(end)
                .arg(operation)
                .readonly(),
        )
    }

    /// Insert values into a ring buffer of `size` slots.
    ///
    /// Each value lands at `insert_index % size`, so once the window is full
    /// the newest values overwrite the oldest. A `size` smaller than the
    /// current window truncates the array to fit.
    ///
    /// # Return
    /// The last index that was written.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arring/>](https://redis.io/commands/arring/)
    #[must_use]
    fn arring(
        self,
        key: impl Serialize,
        size: usize,
        values: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARRING").key(key).arg(size).arg(values))
    }

    /// Iterate the non-empty elements of the inclusive range `[start, end]`.
    ///
    /// # Return
    /// The `(index, value)` pairs that exist in the range — `Vec<(usize, String)>`.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arscan/>](https://redis.io/commands/arscan/)
    #[must_use]
    fn arscan<R: DeserializeOwned>(
        self,
        key: impl Serialize,
        start: usize,
        end: usize,
        limit: impl Into<Option<usize>>,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("ARSCAN")
                .key(key)
                .arg(start)
                .arg(end)
                .arg_labeled("LIMIT", limit.into())
                .readonly(),
        )
    }

    /// Move the insert cursor used by [`arinsert`](ArrayCommands::arinsert) and
    /// [`arring`](ArrayCommands::arring).
    ///
    /// # Return
    /// `true` if the cursor was moved, `false` if the key does not exist.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arseek/>](https://redis.io/commands/arseek/)
    #[must_use]
    fn arseek(self, key: impl Serialize, index: usize) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("ARSEEK").key(key).arg(index))
    }

    /// Set one or more values at consecutive indices, starting at `index`.
    ///
    /// # Return
    /// The number of slots that were empty before the call.
    ///
    /// # See Also
    /// [<https://redis.io/commands/arset/>](https://redis.io/commands/arset/)
    #[must_use]
    fn arset(
        self,
        key: impl Serialize,
        index: usize,
        values: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("ARSET").key(key).arg(index).arg(values))
    }
}

/// Options for the [`arinfo`](ArrayCommands::arinfo) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ArInfoOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    full: bool,
}

impl ArInfoOptions {
    /// Also report the per-slice statistics, which costs a walk of every slice.
    #[must_use]
    pub fn full(mut self) -> Self {
        self.full = true;
        self
    }
}

/// Options for the [`arlastitems`](ArrayCommands::arlastitems) command
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ArLastItemsOptions {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    rev: bool,
}

impl ArLastItemsOptions {
    /// Return the most recent element first instead of last.
    #[must_use]
    pub fn rev(mut self) -> Self {
        self.rev = true;
        self
    }
}

/// Aggregate operation of the [`arop`](ArrayCommands::arop) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ArOperation<'a> {
    /// Sum of the numeric values.
    Sum,
    /// Smallest numeric value.
    Min,
    /// Largest numeric value.
    Max,
    /// Bitwise AND, each value truncated towards zero to an integer.
    And,
    /// Bitwise OR, each value truncated towards zero to an integer.
    Or,
    /// Bitwise XOR, each value truncated towards zero to an integer.
    Xor,
    /// Number of elements equal to the given value.
    Match(&'a str),
    /// Number of non-empty elements.
    Used,
}

/// A textual predicate of the [`argrep`](ArrayCommands::argrep) command
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum ArGrepPredicate<'a> {
    /// The value equals the string.
    Exact(&'a str),
    /// The value contains the string.
    Match(&'a str),
    /// The value matches the glob-style pattern, with the `*`, `?` and `[...]`
    /// wildcards of [`SCAN`](crate::commands::GenericCommands::scan)'s `MATCH`.
    Glob(&'a str),
    /// The value matches the regular expression.
    Re(&'a str),
}

/// Predicates and options of the [`argrep`](ArrayCommands::argrep) command
///
/// At least one predicate is required, which is why the type is built from one.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ArGrep<'a> {
    #[serde(rename = "")]
    predicates: SmallVec<[ArGrepPredicate<'a>; 4]>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    combine: Option<ArGrepCombine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<usize>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withvalues: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nocase: bool,
}

/// How [`argrep`](ArrayCommands::argrep) combines several predicates
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum ArGrepCombine {
    And,
    Or,
}

impl<'a> ArGrep<'a> {
    /// Search with a single predicate.
    #[must_use]
    pub fn new(predicate: ArGrepPredicate<'a>) -> Self {
        let mut predicates = SmallVec::new();
        predicates.push(predicate);
        Self {
            predicates,
            combine: None,
            limit: None,
            withvalues: false,
            nocase: false,
        }
    }

    /// Add another predicate. Combined with
    /// [`or`](ArGrep::or) unless [`and`](ArGrep::and) says otherwise.
    #[must_use]
    pub fn predicate(mut self, predicate: ArGrepPredicate<'a>) -> Self {
        self.predicates.push(predicate);
        self
    }

    /// Match only the elements every predicate accepts.
    #[must_use]
    pub fn and(mut self) -> Self {
        self.combine = Some(ArGrepCombine::And);
        self
    }

    /// Match the elements any predicate accepts. This is the default.
    #[must_use]
    pub fn or(mut self) -> Self {
        self.combine = Some(ArGrepCombine::Or);
        self
    }

    /// Stop after this many matches.
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Return each matching value next to its index.
    #[must_use]
    pub fn with_values(mut self) -> Self {
        self.withvalues = true;
        self
    }

    /// Compare without regard to case.
    #[must_use]
    pub fn nocase(mut self) -> Self {
        self.nocase = true;
        self
    }
}

/// Result for the [`arinfo`](ArrayCommands::arinfo) command
///
/// The last five fields are reported only under
/// [`full`](ArInfoOptions::full).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ArrayInfo {
    /// Number of non-empty elements.
    pub count: usize,
    /// Highest index in use, plus one.
    pub len: usize,
    /// Index [`arinsert`](ArrayCommands::arinsert) would write next.
    pub next_insert_index: usize,
    /// Number of slices the elements are spread over.
    pub slices: usize,
    /// Number of entries in the slice directory.
    pub directory_size: usize,
    /// Number of entries in the super directory.
    pub super_dir_entries: usize,
    /// Number of slots a slice holds.
    pub slice_size: usize,
    /// Slices stored as a plain run of slots.
    #[serde(default)]
    pub dense_slices: Option<usize>,
    /// Slices stored as index/value entries, which is what a gap-heavy region
    /// collapses to.
    #[serde(default)]
    pub sparse_slices: Option<usize>,
    /// Mean number of slots of a dense slice.
    #[serde(default)]
    pub avg_dense_size: Option<f64>,
    /// Mean fraction of a dense slice that holds a value.
    #[serde(default)]
    pub avg_dense_fill: Option<f64>,
    /// Mean number of entries of a sparse slice.
    #[serde(default)]
    pub avg_sparse_size: Option<f64>,
}
