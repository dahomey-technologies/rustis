use crate::{
    client::{PreparedCommand, prepare_command},
    resp::{Args, BulkString, CommandArgs, Response, cmd},
};
use serde::Deserialize;

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
        key: impl Args,
        reduce_dim: Option<usize>,
        values: &[f32],
        element: impl Args,
        options: VAddOptions,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(
            self,
            cmd("VADD")
                .arg(key)
                .arg(reduce_dim)
                .arg("FP32")
                .arg(to_fp32(values))
                .arg(element)
                .arg(options),
        )
    }

    /// Return the number of elements in the specified vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vcard/>](https://redis.io/commands/vcard/)
    #[must_use]
    fn vcard(self, key: impl Args) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("VCARD").arg(key))
    }

    /// Return the number of dimensions of the vectors in the specified vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vdim/>](https://redis.io/commands/vdim/)
    #[must_use]
    fn vdim(self, key: impl Args) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("VDIM").arg(key))
    }

    /// Return the approximate vector associated with a given element in the vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vemb/>](https://redis.io/commands/vemb/)
    #[must_use]
    fn vemb<R: Response>(self, key: impl Args, element: impl Args) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VEMB").arg(key).arg(element))
    }

    /// Return the JSON attributes associated with an element in a vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vemb/>](https://redis.io/commands/vemb/)
    #[must_use]
    fn vgetattr<R: Response>(
        self,
        key: impl Args,
        element: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VGETATTR").arg(key).arg(element))
    }

    /// Return metadata and internal details about a vector set,
    /// including size, dimensions, quantization type, and graph structure.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vinfo/>](https://redis.io/commands/vinfo/)
    #[must_use]
    fn vinfo(self, key: impl Args) -> PreparedCommand<'a, Self, VInfoResult> {
        prepare_command(self, cmd("VINFO").arg(key))
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
        key: impl Args,
        element: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VLINKS").arg(key).arg(element))
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
        key: impl Args,
        element: impl Args,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VLINKS").arg(key).arg(element))
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
        key: impl Args,
        count: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("VRANDMEMBER").arg(key).arg(count))
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
    fn vrem(self, key: impl Args, element: impl Args) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("VREM").arg(key).arg(element))
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
        key: impl Args,
        element: impl Args,
        json: impl Args,
    ) -> PreparedCommand<'a, Self, bool> {
        prepare_command(self, cmd("VSETATTR").arg(key).arg(element).arg(json))
    }

    /// Return elements similar to a given vector or element.
    /// Use this command to perform approximate or exact similarity searches within a vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vsim/>](https://redis.io/commands/vsim/)
    #[must_use]
    fn vsim<R: Response>(
        self,
        key: impl Args,
        vector_or_element: VectorOrElement,
        options: VSimOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("VSIM").arg(key).arg(vector_or_element).arg(options),
        )
    }

    /// Return elements similar to a given vector or element.
    /// Use this command to perform approximate or exact similarity searches within a vector set.
    ///
    /// # See Also
    /// [<https://redis.io/commands/vsim/>](https://redis.io/commands/vsim/)
    #[must_use]
    fn vsim_with_scores<R: Response>(
        self,
        key: impl Args,
        vector_or_element: VectorOrElement,
        options: VSimOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("VSIM")
                .arg(key)
                .arg(vector_or_element)
                .arg("WITHSCORES")
                .arg(options),
        )
    }
}

fn to_fp32(values: &[f32]) -> BulkString {
    let mut buf = Vec::with_capacity(values.len() * 4);
    for f in values {
        buf.extend_from_slice(&f.to_le_bytes()); // little endian
    }
    buf.into()
}

/// Options for the [`vadd`](VectorSetCommands::vadd) command.
#[derive(Default)]
pub struct VAddOptions {
    command_args: CommandArgs,
}

impl VAddOptions {
    /// performs the operation partially using threads, in a check-and-set style.
    /// The neighbor candidates collection, which is slow, is performed in the background,
    ///  while the command is executed in the main thread.
    #[must_use]
    pub fn cas(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("CAS").build(),
        }
    }

    /// in the first VADD call for a given key,
    /// NOQUANT forces the vector to be created without int8 quantization,
    /// which is otherwise the default.
    #[must_use]
    pub fn noquant(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOQUANT").build(),
        }
    }

    /// forces the vector to use binary quantization instead of int8.
    /// This is much faster and uses less memory, but impacts the recall quality.
    #[must_use]
    pub fn bin(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("BIN").build(),
        }
    }

    /// forces the vector to use signed 8-bit quantization.
    /// This is the default, and the option only exists to make sure to check at insertion time
    /// that the vector set is of the same format.
    #[must_use]
    pub fn q8(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("Q8").build(),
        }
    }

    /// plays a role in the effort made to find good candidates when connecting the new node
    /// to the existing Hierarchical Navigable Small World (HNSW) graph. The default is 200.
    /// Using a larger value may help in achieving a better recall.
    /// To improve the recall it is also possible to increase EF during VSIM searches.
    #[must_use]
    pub fn ef(mut self, build_exploration_factor: u32) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("EF")
                .arg(build_exploration_factor)
                .build(),
        }
    }

    /// associates attributes in the form of a JavaScript object to the newly created entry
    /// or updates the attributes (if they already exist).
    /// It is the same as calling the VSETATTR command separately.
    #[must_use]
    pub fn setattr(mut self, attributes: &str) -> Self {
        Self {
            command_args: self.command_args.arg("SETATTR").arg(attributes).build(),
        }
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
    pub fn m(mut self, numlinks: usize) -> Self {
        Self {
            command_args: self.command_args.arg("SMETATTR").arg(numlinks).build(),
        }
    }
}

impl Args for VAddOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        self.command_args.write_args(args);
    }
}

/// Result for the [`vinfo`](VectorSetCommands::vinfo) command.
#[derive(Debug, Deserialize)]
pub struct VInfoResult {
    #[serde(rename = "quant-type")]
    pub quant_type: String,
    #[serde(rename = "vector-dim")]
    pub vector_dim: usize,
    pub size: usize,
    #[serde(rename = "max-level")]
    pub max_level: usize,
    #[serde(rename = "vset-uid")]
    pub vset_uid: u32,
    #[serde(rename = "hnsw-max-node-uid")]
    pub hnsw_max_node_uid: u32,
}

/// Argument of the [`vsim`](VectorSetCommands::vsim) command
pub enum VectorOrElement<'a> {
    Vector(&'a [f32]),
    Element(&'a str),
}

impl<'a> Args for VectorOrElement<'a> {
    fn write_args(&self, args: &mut CommandArgs) {
        match self {
            VectorOrElement::Vector(vector) => {
                args.arg("FP32").arg(to_fp32(vector));
            }
            VectorOrElement::Element(element) => {
                args.arg("ELE").arg(*element);
            }
        }
    }
}

/// Options for the [`vsim`](VectorSetCommands::vsim) command.
#[derive(Default)]
pub struct VSimOptions {
    command_args: CommandArgs,
}

impl VSimOptions {
    /// Limits the number of returned results to num.
    #[must_use]
    pub fn count(mut self, num: usize) -> Self {
        Self {
            command_args: self.command_args.arg("COUNT").arg(num).build(),
        }
    }

    /// Controls the search effort.
    ///
    /// Higher values explore more nodes, improving recall at the cost of speed.
    /// Typical values range from 50 to 1000.
    #[must_use]
    pub fn ef(mut self, search_exploration_factor: u32) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("EF")
                .arg(search_exploration_factor)
                .build(),
        }
    }

    /// Applies a filter expression to restrict matching elements.
    /// See the filtered search section for syntax details.
    #[must_use]
    pub fn filter(mut self, expression: &str) -> Self {
        Self {
            command_args: self.command_args.arg("FILTER").arg(expression).build(),
        }
    }

    /// Limits the number of filtering attempts for the FILTER expression.
    ///
    /// See the [filtered search](https://redis.io/docs/data-types/vector-sets/filtered-search/) section for more.
    #[must_use]
    pub fn filter_ef(mut self, max_filtering_effort: usize) -> Self {
        Self {
            command_args: self
                .command_args
                .arg("FILTER-EF")
                .arg(max_filtering_effort)
                .build(),
        }
    }

    /// Forces an exact linear scan of all elements, bypassing the HNSW graph.
    ///
    /// Use for benchmarking or to calculate recall.
    /// This is significantly slower (O(N)).
    #[must_use]
    pub fn truth(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("TRUTH").build(),
        }
    }

    /// Executes the search in the main thread instead of a background thread.
    ///
    /// Useful for small vector sets or benchmarks.
    /// This may block the server during execution.
    #[must_use]
    pub fn nothread(mut self) -> Self {
        Self {
            command_args: self.command_args.arg("NOTHREAD").build(),
        }
    }
}

impl Args for VSimOptions {
    fn write_args(&self, args: &mut CommandArgs) {
        self.command_args.write_args(args);
    }
}
