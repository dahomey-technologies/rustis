use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Response, cmd, serialize_flag},
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// A group of Redis commands related to [`Vector Sets`](https://redis.io/docs/data-types/vector-sets/)
///
/// # See Also
/// [Redis Sorted Set Commands](https://redis.io/docs/latest/commands/?group=vector_set)
pub trait VectorSetCommands<'a>: Sized {
    /// Add a new element into the vector set specified by key.
    ///
    /// The vector can be provided as 32-bit floating point (FP32) blob of values,
    /// or as floating point numbers as strings
    ///
    /// # Arguments
    /// * `key` - is the name of the key that will hold the vector set data.
    /// * `reduce_dim` - implements random projection to reduce the dimensionality of the vector.
    ///   The projection matrix is saved and reloaded along with the vector set.
    /// * `values` - vector values.
    /// * `element` - is the name of the element that is being added to the vector set.
    ///
    /// # Return
    /// * `true` - if key was added.
    /// * `false` - if key was not added.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vadd/>](https://redis.io/commands/vadd/)
    #[must_use]
    fn vadd(
        self,
        key: impl Serialize,
        reduce_dim: impl Into<Option<u32>>,
        values: &[f32],
        element: impl Serialize,
        options: VAddOptions,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("VADD")
                .key(key)
                .arg(reduce_dim.into())
                .arg("FP32")
                .arg(Fp32Vector(values))
                .arg(element)
                .arg(options),
        )
    }

    /// Return the number of elements in the specified vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vcard/>](https://redis.io/commands/vcard/)
    #[must_use]
    fn vcard(self, key: impl Serialize) -> PreparedCommand<'a, Self, u32> {
        prepare_command(self, cmd("VCARD").key(key))
    }

    /// Return the number of dimensions of the vectors in the specified vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vdim/>](https://redis.io/commands/vdim/)
    #[must_use]
    fn vdim(self, key: impl Serialize) -> PreparedCommand<'a, Self, u32> {
        prepare_command(self, cmd("VDIM").key(key))
    }

    /// Return the approximate vector associated with a given element in the vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vemb/>](https://redis.io/commands/vemb/)
    #[must_use]
    fn vemb<R: Response>(
        self,
        key: impl Serialize,
        element: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VEMB").key(key).arg(element))
    }

    /// Return the JSON attributes associated with an element in a vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vemb/>](https://redis.io/commands/vemb/)
    #[must_use]
    fn vgetattr<R: Response>(
        self,
        key: impl Serialize,
        element: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VGETATTR").key(key).arg(element))
    }

    /// Return metadata and internal details about a vector set,
    /// including size, dimensions, quantization type, and graph structure.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vinfo/>](https://redis.io/commands/vinfo/)
    #[must_use]
    fn vinfo(self, key: impl Serialize) -> PreparedCommand<'a, Self, VInfoResult> {
        prepare_command(self, cmd("VINFO").key(key))
    }

    /// Return the neighbors of a specified element in a vector set.
    /// The command shows the connections for each layer of the HNSW graph.
    ///
    /// # Return
    /// a collection containing the names of adjacent elements
    ///
    /// # See Also
    /// [<https://redis.io/commands/vlinks/>](https://redis.io/commands/vlinks/)
    #[must_use]
    fn vlinks<R: Response>(
        self,
        key: impl Serialize,
        element: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VLINKS").key(key).arg(element))
    }

    /// Return the neighbors of a specified element in a vector set.
    /// The command shows the connections for each layer of the HNSW graph.
    ///
    /// # Return
    /// a collection containing the names of adjacent elements
    /// together with their scores as doubles
    ///
    /// # See Also
    /// [<https://redis.io/commands/vlinks/>](https://redis.io/commands/vlinks/)
    #[must_use]
    fn vlinks_with_score<R: Response>(
        self,
        key: impl Serialize,
        element: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VLINKS").key(key).arg(element))
    }

    /// Return one or more random elements from a vector set.
    ///
    /// The behavior is similar to the SRANDMEMBER command:
    /// * When called without a count, returns a single element as a bulk string.
    /// * When called with a positive count,
    ///   returns up to that many distinct elements (no duplicates).
    /// * When called with a negative count, returns that many elements,
    ///   possibly with duplicates.
    /// * If the count exceeds the number of elements, the entire set is returned.
    /// * If the key does not exist, the command returns null if no count is given,
    ///   or an empty array if a count is provided.
    ///
    /// # Return
    /// a collecton containing the names of count random elements as strings.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vrandmember/>](https://redis.io/commands/vrandmember/)
    #[must_use]
    fn vrandmember<R: Response>(
        self,
        key: impl Serialize,
        count: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VRANDMEMBER").key(key).arg(count))
    }

    /// Remove an element from a vector set.
    ///
    /// # Return
    /// * `true` - if the element was removed.
    /// * `false` - if either element or key do not exist.
    ///
    /// VREM reclaims memory immediately.
    /// It does not use tombstones or logical deletions,
    /// making it safe to use in long-running applications
    /// that frequently update the same vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vrem/>](https://redis.io/commands/vrem/)
    #[must_use]
    fn vrem(self, key: impl Serialize, element: impl Serialize) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("VREM").key(key).arg(element))
    }

    /// Associate a JSON object with an element in a vector set.
    ///
    /// Use this command to store attributes that can be used in filtered similarity searches with VSIM.
    ///
    /// You can also update existing attributes or delete them by setting an empty string.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vemb/>](https://redis.io/commands/vemb/)
    #[must_use]
    fn vsetattr(
        self,
        key: impl Serialize,
        element: impl Serialize,
        json: impl Serialize,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("VSETATTR").key(key).arg(element).arg(json))
    }

    /// Return elements similar to a given vector or element.
    /// Use this command to perform approximate or exact similarity searches within a vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vsim/>](https://redis.io/commands/vsim/)
    #[must_use]
    fn vsim<R: Response>(
        self,
        key: impl Serialize,
        vector_or_element: VectorOrElement,
        options: VSimOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("VSIM").key(key).arg(vector_or_element).arg(options),
        )
    }
}

struct Fp32Vector<'a>(&'a [f32]);

impl<'a> Serialize for Fp32Vector<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut buf = SmallVec::<[u8; 64]>::with_capacity(self.0.len() * 4);
        for f in self.0 {
            buf.extend_from_slice(&f.to_le_bytes()); // little endian
        }
        serializer.serialize_bytes(&buf)
    }
}

/// Options for the [`vadd`](VectorSetCommands::vadd) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct VAddOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    cas: bool,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    quantization: Option<QuantizationOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ef: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    setattr: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    m: Option<u32>,
}

impl<'a> VAddOptions<'a> {
    /// performs the operation partially using threads, in a check-and-set style.
    /// The neighbor candidates collection, which is slow, is performed in the background,
    ///  while the command is executed in the main thread.
    #[must_use]
    pub fn cas(mut self) -> Self {
        self.cas = true;
        self
    }

    /// in the first VADD call for a given key,
    /// NOQUANT forces the vector to be created without int8 quantization,
    /// which is otherwise the default.
    #[must_use]
    pub fn quantization(mut self, quantization: QuantizationOptions) -> Self {
        self.quantization = Some(quantization);
        self
    }

    /// plays a role in the effort made to find good candidates when connecting the new node
    /// to the existing Hierarchical Navigable Small World (HNSW) graph. The default is 200.
    /// Using a larger value may help in achieving a better recall.
    /// To improve the recall it is also possible to increase EF during VSIM searches.
    #[must_use]
    pub fn ef(mut self, build_exploration_factor: u32) -> Self {
        self.ef = Some(build_exploration_factor);
        self
    }

    /// associates attributes in the form of a JavaScript object to the newly created entry
    /// or updates the attributes (if they already exist).
    /// It is the same as calling the VSETATTR command separately.
    #[must_use]
    pub fn set_attr(mut self, attributes: &'a str) -> Self {
        self.setattr = Some(attributes);
        self
    }

    /// is the maximum number of connections that each node of the graph
    /// will have with other nodes.
    /// The default is 16. More connections means more memory, but provides for more efficient
    /// graph exploration.
    /// Nodes at layer zero (every node exists at least at layer zero) have M * 2 connections,
    /// while the other layers only have M connections.
    /// For example, setting M to 64 will use at least 1024 bytes of memory for layer zero.
    ///  That's M * 2 connections times 8 bytes (pointers), or 128 * 8 = 1024. For higher layers,
    /// consider the following:
    ///
    /// * Each node appears in ~1.33 layers on average (empirical observation from HNSW papers),
    ///   which works out to be 0.33 higher layers per node.
    /// * Each of those higher layers has M = 64 connections.
    ///
    /// So, the additional amount of memory is approximately 0.33 × 64 × 8 ≈ 169.6 bytes per node,
    /// bringing the total memory to ~1193 bytes.
    ///
    /// If you don't have a recall quality problem, the default is acceptable,
    /// and uses a minimal amount of memory.
    #[must_use]
    pub fn m(mut self, num_links: u32) -> Self {
        self.m = Some(num_links);
        self
    }
}

/// Quantization options for [`vadd`](VectorSetCommands::vadd) command.
#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum QuantizationOptions {
    /// in the first VADD call for a given key, NOQUANT forces the vector to be created without int8 quantization,
    /// which is otherwise the default.
    NoQuant,
    /// forces the vector to use binary quantization instead of int8.
    /// This is much faster and uses less memory, but impacts the recall quality.
    Bin,
    /// forces the vector to use signed 8-bit quantization.
    /// This is the default, and the option only exists to make sure to check
    /// at insertion time that the vector set is of the same format.
    Q8,
}

/// Result for the [`vinfo`](VectorSetCommands::vinfo) command.
#[derive(Debug, Deserialize)]
pub struct VInfoResult {
    #[serde(rename = "quant-type")]
    pub quant_type: String,
    #[serde(rename = "vector-dim")]
    pub vector_dim: u32,
    pub size: u32,
    #[serde(rename = "max-level")]
    pub max_level: u32,
    #[serde(rename = "vset-uid")]
    pub vset_uid: u32,
    #[serde(rename = "hnsw-max-node-uid")]
    pub hnsw_max_node_uid: u32,
}

/// Argument of the [`vsim`](VectorSetCommands::vsim) command
#[derive(Serialize)]
pub enum VectorOrElement<'a> {
    #[serde(rename = "FP32")]
    Vector(&'a [f32]),
    #[serde(rename = "ELE")]
    Element(&'a str),
}

/// Options for the [`vsim`](VectorSetCommands::vsim) command.
#[derive(Default, Serialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub struct VSimOptions<'a> {
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withscores: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    withattributes: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    epsilon: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ef: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filter_ef: Option<u32>,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    truth: bool,
    #[serde(
        skip_serializing_if = "std::ops::Not::not",
        serialize_with = "serialize_flag"
    )]
    nothread: bool,
}

impl<'a> VSimOptions<'a> {
    /// returns the similarity score (from 1 to 0) alongside each result.
    /// A score of 1 is identical; 0 is the opposite.
    #[must_use]
    pub fn with_scores(mut self) -> Self {
        self.withscores = true;
        self
    }

    /// returns, for each element, the JSON attribute associated with the element
    /// or NULL when no attributes are present.
    #[must_use]
    pub fn with_attributes(mut self) -> Self {
        self.withattributes = true;
        self
    }

    /// Limits the number of returned results to num.
    #[must_use]
    pub fn count(mut self, num: u32) -> Self {
        self.count = Some(num);
        self
    }

    /// is a floating point number between 0 and 1.
    ///  It is used to retrieve elements that have a distance that is no further than the specified delta.
    /// In vector sets, returned elements have a similarity score (when compared to the query vector)
    /// that is between 1 and 0, where 1 means identical and 0 means opposite vectors.
    /// For example, if the EPSILON option is specified with an argument of 0.2,
    /// it means only elements that have a similarity of 0.8 or better (a distance < 0.2) are returned.
    /// This is useful when you specify a large COUNT,
    /// but you don't want elements that are too far away from the query vector.
    #[must_use]
    pub fn epsilon(mut self, delta: f32) -> Self {
        self.epsilon = Some(delta);
        self
    }
    /// Controls the search effort.
    ///
    /// Higher values explore more nodes, improving recall at the cost of speed.
    /// Typical values range from 50 to 1000.
    #[must_use]
    pub fn ef(mut self, search_exploration_factor: u32) -> Self {
        self.ef = Some(search_exploration_factor);
        self
    }

    /// Applies a filter expression to restrict matching elements.
    /// See the filtered search section for syntax details.
    #[must_use]
    pub fn filter(mut self, expression: &'a str) -> Self {
        self.filter = Some(expression);
        self
    }

    /// Limits the number of filtering attempts for the FILTER expression.
    ///
    /// See the [filtered search](https://redis.io/docs/data-types/vector-sets/filtered-search/) section for more.
    #[must_use]
    pub fn filter_ef(mut self, max_filtering_effort: u32) -> Self {
        self.filter_ef = Some(max_filtering_effort);
        self
    }

    /// Forces an exact linear scan of all elements, bypassing the HNSW graph.
    ///
    /// Use for benchmarking or to calculate recall.
    /// This is significantly slower (O(N)).
    #[must_use]
    pub fn truth(mut self) -> Self {
        self.truth = true;
        self
    }

    /// Executes the search in the main thread instead of a background thread.
    ///
    /// Useful for small vector sets or benchmarks.
    /// This may block the server during execution.
    #[must_use]
    pub fn no_thread(mut self) -> Self {
        self.nothread = true;
        self
    }
}
