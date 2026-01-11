use crate::{
    client::{PreparedCommand, prepare_command},
    commands::SetCondition,
    resp::{CommandArgsMut, Response, cmd},
};
use serde::Serialize;

/// A group of Redis commands related to [`RedisJson`](https://redis.io/docs/stack/json/)
///
/// # See Also
/// [RedisJson Commands](https://redis.io/commands/?group=json)
pub trait JsonCommands<'a>: Sized {
    /// Append the json `values` into the array at `path` after the last element in it
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `values` - one or more values to append to one or more arrays.
    ///
    /// # Return
    /// A collection of integer replies for each path, the array's new size,
    /// or nil, if the matching JSON value is not an array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.arrappend/>](https://redis.io/commands/json.arrappend/)
    #[must_use]
    fn json_arrappend<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        values: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.ARRAPPEND").key(key).arg(path).arg(values))
    }

    /// Search for the first occurrence of a scalar JSON value in an array
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    /// * `value` - value index to find in one or more arrays.
    ///
    /// # Return
    /// A collection of integer replies for each path,
    ///
    /// the first position in the array of each JSON value that matches the path,
    /// -1 if unfound in the array, or nil, if the matching JSON value is not an array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.arrindex/>](https://redis.io/commands/json.arrindex/)
    #[must_use]
    fn json_arrindex<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        value: impl Serialize,
        options: JsonArrIndexOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("JSON.ARRINDEX")
                .key(key)
                .arg(path)
                .arg(value)
                .arg(options),
        )
    }

    /// Insert the json `values` into the array at `path` before the `index` (shifts to the right)
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `index`- The position in the array where you want to insert a value.
    ///   The index must be in the array's range.
    ///   Inserting at index 0 prepends to the array.
    ///   Negative index values start from the end of the array.
    /// * `values` - one or more values to insert in one or more arrays.
    ///
    /// # Return
    /// A collection of integer replies for each path,
    /// the array's new size, or nil,
    /// if the matching JSON value is not an array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.arrinsert/>](https://redis.io/commands/json.arrinsert/)
    #[must_use]
    fn json_arrinsert<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        index: isize,
        values: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("JSON.ARRINSERT")
                .key(key)
                .arg(path)
                .arg(index)
                .arg(values),
        )
    }

    /// Report the length of the JSON array at `path` in `key`
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// A collection of integer replies, an integer for each matching value,
    /// each is the array's length, or nil, if the matching value is not an array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.arrlen/>](https://redis.io/commands/json.arrlen/)
    #[must_use]
    fn json_arrlen<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.ARRLEN").key(key).arg(path))
    }

    /// Remove and return an element from the index in the array
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `index`- is position in the array to start popping from.
    ///   Default is -1, meaning the last element.
    ///   Out-of-range indexes round to their respective array ends.
    ///   Popping an empty array returns null.
    ///
    /// # Return
    /// A collection of bulk string replies for each path, each reply is the popped JSON value,
    /// or nil, if the matching JSON value is not an array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.arrpop/>](https://redis.io/commands/json.arrpop/)
    #[must_use]
    fn json_arrpop<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        index: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.ARRPOP").key(key).arg(path).arg(index))
    }

    /// Remove and return an element from the index in the array
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `start`- The index of the first element to keep (previous elements are trimmed).
    /// * `stop` - the index of the last element to keep (following elements are trimmed), including the last element.
    ///   Negative values are interpreted as starting from the end.
    ///
    /// # Return
    /// A collection of integer replies for each path, the array's new size,
    /// or nil, if the matching JSON value is not an array.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.arrtrim/>](https://redis.io/commands/json.arrtrim/)
    #[must_use]
    fn json_arrtrim<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        start: isize,
        stop: isize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(
            self,
            cmd("JSON.ARRTRIM").key(key).arg(path).arg(start).arg(stop),
        )
    }

    /// Clear container values (arrays/objects) and set numeric values to 0
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// The number of values cleared.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.clear/>](https://redis.io/commands/json.clear/)
    #[must_use]
    fn json_clear(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("JSON.CLEAR").key(key).arg(path))
    }

    /// Report a value's memory usage in bytes
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    ///  A collection of integer replies for each path, the value size in bytes
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.debug-memory/>](https://redis.io/commands/json.debug-memory/)
    #[must_use]
    fn json_debug_memory<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.DEBUG").arg("MEMORY").key(key).arg(path))
    }

    /// Delete a value
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    ///  The number of paths deleted (0 or more).
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.del/>](https://redis.io/commands/json.del/)
    #[must_use]
    fn json_del(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("JSON.DEL").key(key).arg(path))
    }

    /// See [`json_del`](JsonCommands::json_del)
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    ///  The number of paths deleted (0 or more).
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.forget/>](https://redis.io/commands/json.forget/)
    #[must_use]
    fn json_forget(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, usize> {
        prepare_command(self, cmd("JSON.FORGET").key(key).arg(path))
    }

    /// Return the value at path in JSON serialized form
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `options`- See [`JsonOptions`](JsonGetOptions)
    ///
    /// # Return
    /// A collection of bulk string replies. Each string is the JSON serialization of each JSON value that matches a path
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.get/>](https://redis.io/commands/json.get/)
    #[must_use]
    fn json_get<R: Response>(
        self,
        key: impl Serialize,
        options: JsonGetOptions,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.GET").key(key).arg(options))
    }

    /// Return the values at `path` from multiple `key` arguments
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `options`- See [`JsonOptions`](JsonGetOptions)
    ///
    /// # Return
    /// A collection of bulk string replies specified as the JSON serialization of the value at each key's path.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.mget/>](https://redis.io/commands/json.mget/)
    #[must_use]
    fn json_mget<R: Response>(
        self,
        keys: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.MGET").key(keys).arg(path))
    }

    /// Set or update one or more JSON values according to the specified key-path-value triplets
    ///
    /// # Arguments
    /// key-path-value triplets
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.mset/>](https://redis.io/commands/json.mset/)
    #[must_use]
    fn json_mset(self, key_path_values: impl Serialize) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("JSON.MSET")
                .key_with_step(key_path_values, 3)
                .cluster_info(None, None, 3),
        )
    }

    /// Increment the number value stored at path by number
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `value` - number value to increment.
    ///
    /// # Return
    /// A bulk string reply specified as a stringified new value for each path,
    /// or nil, if the matching JSON value is not a number.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.numincrby/>](https://redis.io/commands/json.numincrby/)
    #[must_use]
    fn json_numincrby<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.NUMINCRBY").key(key).arg(path).arg(value))
    }

    /// Multiply the number value stored at path by number
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `value` - number value to increment.
    ///
    /// # Return
    /// A bulk string reply specified as a stringified new value for each path,
    /// or nil, if the matching JSON value is not a number.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.nummultby/>](https://redis.io/commands/json.nummultby/)
    #[must_use]
    fn json_nummultby<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.NUMMULTBY").key(key).arg(path).arg(value))
    }

    /// Return the keys in the object that's referenced by `path`
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// A collection of collection replies for each path,
    /// a collection of the key names in the object as a bulk string reply,
    /// or an empty collection if the matching JSON value is not an object.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.objkeys/>](https://redis.io/commands/json.objkeys/)
    #[must_use]
    fn json_objkeys<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.OBJKEYS").key(key).arg(path))
    }

    /// Report the number of keys in the JSON object at `path` in `key`
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// A collection of integer replies for each path specified as the number of keys in the object or nil,
    /// if the matching JSON value is not an object.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.objlen/>](https://redis.io/commands/json.objlen/)
    #[must_use]
    fn json_objlen<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.OBJLEN").key(key).arg(path))
    }

    /// Return the JSON in key in
    /// [`Redis serialization protocol specification`](https://redis.io/docs/reference/protocol-spec) form
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// A collection of [`Values`](crate::resp::Value)
    ///
    /// This command uses the following mapping from JSON to RESP:
    /// * JSON `null` maps to the bulk string reply.
    /// * JSON `false` and `true` values map to the simple string reply.
    /// * JSON number maps to the integer reply or bulk string reply, depending on type.
    /// * JSON string maps to the bulk string reply.
    /// * JSON array is represented as an array reply in which the first element is the simple string reply `[`, followed by the array's elements.
    /// * JSON object is represented as an array reply in which the first element is the simple string reply `{`.
    ///   Each successive entry represents a key-value pair as a two-entry array reply of the bulk string reply.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.resp/>](https://redis.io/commands/json.resp/)
    #[must_use]
    fn json_resp<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.RESP").key(key).arg(path))
    }

    /// Set the JSON value at `path` in `key`
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path` - JSONPath to specify.\
    ///   For new Redis keys the path must be the root.\
    ///   For existing keys, when the entire path exists, the value that it contains is replaced with the json value.\
    ///   For existing keys, when the path exists, except for the last element, a new child is added with the json value.
    /// * `value`- The value to set at the specified path
    /// * `condition`- See [`SetCondition`](crate::commands::SetCondition)
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.set/>](https://redis.io/commands/json.set/)
    #[must_use]
    fn json_set<'b>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        value: impl Serialize,
        condition: impl Into<Option<SetCondition<'b>>>,
    ) -> PreparedCommand<'a, Self, ()> {
        prepare_command(
            self,
            cmd("JSON.SET")
                .key(key)
                .arg(path)
                .arg(value)
                .arg(condition.into()),
        )
    }

    /// Append the json-string values to the string at path
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    /// * `value` - number value to increment.
    ///
    /// # Return
    /// A collection of integer replies for each path, the string's new length, or nil, if the matching JSON value is not a string.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.strappend/>](https://redis.io/commands/json.strappend/)
    #[must_use]
    fn json_strappend<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
        value: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.STRAPPEND").key(key).arg(path).arg(value))
    }

    /// Report the length of the JSON String at `path` in `key`
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// returns by recursive descent a collection of integer replies for each path,
    /// the array's length, or nil, if the matching JSON value is not a string.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.strlen/>](https://redis.io/commands/json.strlen/)
    #[must_use]
    fn json_strlen<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.STRLEN").key(key).arg(path))
    }

    /// Toggle a Boolean value stored at `path`
    ///
    /// # Arguments
    /// * `key` - The key to modify.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// A collection of integer replies for each path, the new value (0 if false or 1 if true),
    /// or nil for JSON values matching the path that are not Boolean.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.toggle/>](https://redis.io/commands/json.toggle/)
    #[must_use]
    fn json_toggle<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.TOGGLE").key(key).arg(path))
    }

    /// Report the type of JSON value at `path`
    ///
    /// # Arguments
    /// * `key` - The key to parse.
    /// * `path`- The JSONPath to specify.
    ///
    /// # Return
    /// A collection of string replies for each path, specified as the value's type.
    ///
    /// # See Also
    /// [<https://redis.io/commands/json.type/>](https://redis.io/commands/json.type/)
    #[must_use]
    fn json_type<R: Response>(
        self,
        key: impl Serialize,
        path: impl Serialize,
    ) -> PreparedCommand<'a, Self, R> {
        prepare_command(self, cmd("JSON.TYPE").key(key).arg(path))
    }
}

/// Options for the [`json_get`](JsonCommands::json_get) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct JsonGetOptions<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    indent: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    newline: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    space: Option<&'a str>,
    #[serde(rename = "", skip_serializing_if = "CommandArgsMut::is_empty")]
    path: CommandArgsMut,
}

impl<'a> JsonGetOptions<'a> {
    /// Sets the indentation string for nested levels.
    #[must_use]
    pub fn indent(mut self, indent: &'a str) -> Self {
        self.indent = Some(indent);
        self
    }

    /// Sets the string that's printed at the end of each line.
    #[must_use]
    pub fn newline(mut self, newline: &'a str) -> Self {
        self.newline = Some(newline);
        self
    }

    /// Sets the string that's put between a key and a value.
    #[must_use]
    pub fn space(mut self, space: &'a str) -> Self {
        self.space = Some(space);
        self
    }

    /// JSONPath to specify
    #[must_use]
    pub fn path(mut self, paths: impl Serialize) -> Self {
        self.path = self.path.arg(paths);
        self
    }
}

/// Options for the [`json_arrindex`](JsonCommands::json_arrindex) command
#[derive(Default, Serialize)]
#[serde(rename_all(serialize = "UPPERCASE"))]
pub struct JsonArrIndexOptions {
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    start: Option<u32>,
    #[serde(rename = "", skip_serializing_if = "Option::is_none")]
    stop: Option<i32>,
}

impl JsonArrIndexOptions {
    /// Inclusive start value to specify in a slice of the array to search.
    ///
    /// Default is 0.
    #[must_use]
    pub fn start(mut self, start: u32) -> Self {
        self.start = Some(start);
        self
    }

    /// Exclusive stop value to specify in a slice of the array to search, including the last element.
    ///
    /// Default is 0.
    /// Negative values are interpreted as starting from the end.
    #[must_use]
    pub fn stop(mut self, stop: i32) -> Self {
        self.stop = Some(stop);
        self
    }
}
