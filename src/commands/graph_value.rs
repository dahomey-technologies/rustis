use crate::{commands::GraphCache, resp::deserialize_byte_buf, Error, Result};
use serde::{
    de::{self, DeserializeSeed, Unexpected, Visitor},
    Deserialize, Deserializer,
};
use std::{collections::HashMap, fmt, marker::PhantomData};

trait GraphObjectVisitor<'de>: Sized {
    fn visit_seq<A>(seq: A, cache: &GraphCache) -> std::result::Result<Self, A::Error>
    where
        A: de::SeqAccess<'de>;

    fn into_seed(cache: &GraphCache) -> GraphObjectSeed<Self> {
        GraphObjectSeed {
            phantom: PhantomData,
            cache,
        }
    }

    fn into_with_type_seed(
        cache: &GraphCache,
        value_type: GraphValueType,
    ) -> GraphObjectWithTypeSeed<Self> {
        GraphObjectWithTypeSeed {
            phantom: PhantomData,
            cache,
            value_type,
        }
    }

    fn into_vec_seed(cache: &GraphCache, value_type: GraphValueType) -> GraphVecSeed<Self> {
        GraphVecSeed {
            phantom: PhantomData,
            cache,
            value_type,
        }
    }
}

struct GraphObjectSeed<'a, T> {
    phantom: PhantomData<T>,
    cache: &'a GraphCache,
}

impl<'de, 'a, T> Visitor<'de> for GraphObjectSeed<'a, T>
where
    T: GraphObjectVisitor<'de>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Graph object")
    }

    fn visit_seq<A>(self, seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        T::visit_seq(seq, self.cache)
    }
}

impl<'de, 'a, T> DeserializeSeed<'de> for GraphObjectSeed<'a, T>
where
    T: GraphObjectVisitor<'de>,
{
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct GraphObjectWithTypeSeed<'a, T> {
    phantom: PhantomData<T>,
    cache: &'a GraphCache,
    value_type: GraphValueType,
}

impl<'de, 'a, T> Visitor<'de> for GraphObjectWithTypeSeed<'a, T>
where
    T: GraphObjectVisitor<'de>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("Graph object for type {:?}", self.value_type))
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        match seq.next_element::<GraphValueType>()? {
            Some(value_type) if value_type == self.value_type => (),
            _ => return Err(de::Error::custom(format!("expected {:?}", self.value_type))),
        }

        let Some(obj) = seq.next_element_seed(T::into_seed(self.cache))? else {
            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
        };

        Ok(obj)
    }
}

impl<'de, 'a, T> DeserializeSeed<'de> for GraphObjectWithTypeSeed<'a, T>
where
    T: GraphObjectVisitor<'de>,
{
    type Value = T;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct GraphVecSeed<'a, T> {
    phantom: PhantomData<T>,
    cache: &'a GraphCache,
    value_type: GraphValueType,
}

impl<'de, 'a, T> Visitor<'de> for GraphVecSeed<'a, T>
where
    T: GraphObjectVisitor<'de>,
{
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("GraphVecSeed")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        struct SubVecSeed<'a, T> {
            phantom: PhantomData<T>,
            cache: &'a GraphCache,
            value_type: GraphValueType,
        }

        impl<'de, 'a, T> Visitor<'de> for SubVecSeed<'a, T>
        where
            T: GraphObjectVisitor<'de>,
        {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Vec<T>")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut vec = if let Some(size_hint) = seq.size_hint() {
                    Vec::with_capacity(size_hint)
                } else {
                    Vec::new()
                };

                while let Some(obj) =
                    seq.next_element_seed(T::into_with_type_seed(self.cache, self.value_type))?
                {
                    vec.push(obj);
                }

                Ok(vec)
            }
        }

        impl<'de, 'a, T> DeserializeSeed<'de> for SubVecSeed<'a, T>
        where
            T: GraphObjectVisitor<'de>,
        {
            type Value = Vec<T>;

            fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_seq(self)
            }
        }

        match seq.next_element::<GraphValueType>()? {
            Some(GraphValueType::Array) => (),
            _ => return Err(de::Error::custom("expected GraphValueType::Array (6)")),
        }

        let Some(vec) = seq.next_element_seed(SubVecSeed {
            phantom: PhantomData,
            cache: self.cache,
            value_type: self.value_type,
        })?
        else {
            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
        };

        Ok(vec)
    }
}

impl<'de, 'a, T> DeserializeSeed<'de> for GraphVecSeed<'a, T>
where
    T: GraphObjectVisitor<'de>,
{
    type Value = Vec<T>;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub(crate) enum GraphValueType {
    Unknown,
    Null,
    String,
    Integer,
    Boolean,
    Double,
    Array,
    Edge,
    Node,
    Path,
    Map,
    Point,
}

impl<'de> Deserialize<'de> for GraphValueType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value_type = u8::deserialize(deserializer)?;
        match value_type {
            0 => Ok(GraphValueType::Unknown),
            1 => Ok(GraphValueType::Null),
            2 => Ok(GraphValueType::String),
            3 => Ok(GraphValueType::Integer),
            4 => Ok(GraphValueType::Boolean),
            5 => Ok(GraphValueType::Double),
            6 => Ok(GraphValueType::Array),
            7 => Ok(GraphValueType::Edge),
            8 => Ok(GraphValueType::Node),
            9 => Ok(GraphValueType::Path),
            10 => Ok(GraphValueType::Map),
            11 => Ok(GraphValueType::Point),
            _ => Err(de::Error::invalid_value(
                Unexpected::Unsigned(value_type as u64),
                &"A u8 between 1 and 11",
            )),
        }
    }
}

/// Object model for the different [`RedisGraph Data Types`](https://redis.io/docs/stack/graph/datatypes/)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum GraphValue {
    /// In RedisGraph, null is used to stand in for an unknown or missing value.
    Null,
    /// RedisGraph strings are Unicode character sequences.
    String(Vec<u8>),
    /// All RedisGraph integers are treated as 64-bit signed integers.
    Integer(i64),
    /// Boolean values are specified as true or false.
    Boolean(bool),
    /// All RedisGraph floating-point values are treated as 64-bit signed doubles.
    Double(f64),
    /// Arrays are ordered lists of elements.
    Array(Vec<GraphValue>),
    /// Relationships are persistent graph elements that connect one node to another.
    Edge(GraphEdge),
    /// Nodes are persistent graph elements that can be connected to each other via relationships.
    Node(GraphNode),
    /// Paths are alternating sequences of nodes and edges, starting and ending with a node.
    Path(GraphPath),
    /// Maps are order-agnostic collections of key-value pairs.
    Map(HashMap<String, GraphValue>),
    /// The Point data type is a set of latitude/longitude coordinates, stored within RedisGraph as a pair of 32-bit floats.
    Point((f32, f32)),
}

impl GraphValue {
    /// A [`GraphValue`](GraphValue) to user type conversion that consumes the input value.
    ///
    /// # Errors
    /// Any parsing error ([`Error::Client`](crate::Error::Client)) due to incompatibility between Value variant and taget type
    pub fn into<T>(self) -> Result<T>
    where
        T: FromGraphValue,
    {
        T::from_graph_value(self)
    }
}

pub(crate) struct GraphValueSeed<'a> {
    value_type: GraphValueType,
    cache: &'a GraphCache,
}

impl<'a> GraphValueSeed<'a> {
    pub fn from_cache(cache: &'a GraphCache) -> Self {
        Self {
            value_type: GraphValueType::Unknown,
            cache,
        }
    }

    pub fn new(value_type: GraphValueType, cache: &'a GraphCache) -> Self {
        Self { value_type, cache }
    }
}

impl<'de, 'a> DeserializeSeed<'de> for GraphValueSeed<'a> {
    type Value = GraphValue;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        pub struct GraphValueVisitor<'a> {
            cache: &'a GraphCache,
        }

        impl<'de, 'a> Visitor<'de> for GraphValueVisitor<'a> {
            type Value = GraphValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("GraphValue")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let Some(value_type) = seq.next_element::<GraphValueType>()? else {
                    return Err(de::Error::invalid_length(0, &"more elements in sequence"));
                };

                let Some(value) =
                    seq.next_element_seed(GraphValueSeed::new(value_type, self.cache))?
                else {
                    return Err(de::Error::invalid_length(1, &"more elements in sequence"));
                };

                Ok(value)
            }
        }

        let value = match self.value_type {
            GraphValueType::Unknown => {
                deserializer.deserialize_seq(GraphValueVisitor { cache: self.cache })?
            }
            GraphValueType::Null => {
                <()>::deserialize(deserializer)?;
                GraphValue::Null
            }
            GraphValueType::String => GraphValue::String(deserialize_byte_buf(deserializer)?),
            GraphValueType::Integer => GraphValue::Integer(i64::deserialize(deserializer)?),
            GraphValueType::Boolean => GraphValue::Boolean(bool::deserialize(deserializer)?),
            GraphValueType::Double => GraphValue::Double(f64::deserialize(deserializer)?),
            GraphValueType::Array => GraphValue::Array(
                GraphValueArraySeed { cache: self.cache }.deserialize(deserializer)?,
            ),
            GraphValueType::Edge => {
                GraphValue::Edge(GraphEdge::into_seed(self.cache).deserialize(deserializer)?)
            }
            GraphValueType::Node => {
                GraphValue::Node(GraphNode::into_seed(self.cache).deserialize(deserializer)?)
            }
            GraphValueType::Path => {
                GraphValue::Path(GraphPath::into_seed(self.cache).deserialize(deserializer)?)
            }
            GraphValueType::Map => {
                GraphValue::Map(GraphValueMapSeed { cache: self.cache }.deserialize(deserializer)?)
            }
            GraphValueType::Point => GraphValue::Point(<(f32, f32)>::deserialize(deserializer)?),
        };

        Ok(value)
    }
}

pub(crate) struct GraphValueArraySeed<'a> {
    pub cache: &'a GraphCache,
}

impl<'de, 'a> Visitor<'de> for GraphValueArraySeed<'a> {
    type Value = Vec<GraphValue>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Vec<GraphValue>")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut values = if let Some(size_hint) = seq.size_hint() {
            Vec::with_capacity(size_hint)
        } else {
            Vec::new()
        };

        while let Some(value) = seq.next_element_seed(GraphValueSeed::from_cache(self.cache))? {
            values.push(value);
        }

        Ok(values)
    }
}

impl<'de, 'a> DeserializeSeed<'de> for GraphValueArraySeed<'a> {
    type Value = Vec<GraphValue>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct GraphValueMapSeed<'a> {
    pub cache: &'a GraphCache,
}

impl<'de, 'a> Visitor<'de> for GraphValueMapSeed<'a> {
    type Value = HashMap<String, GraphValue>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("HashMap<String, GraphValue>")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut values = if let Some(size_hint) = seq.size_hint() {
            HashMap::with_capacity(size_hint / 2)
        } else {
            HashMap::new()
        };

        while let Some(key) = seq.next_element::<String>()? {
            let Some(value) = seq.next_element_seed(GraphValueSeed::from_cache(self.cache))? else {
                return Err(de::Error::custom("Cannot parse GraphValue map"));
            };
            values.insert(key, value);
        }

        Ok(values)
    }
}

impl<'de, 'a> DeserializeSeed<'de> for GraphValueMapSeed<'a> {
    type Value = HashMap<String, GraphValue>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

/// Nodes are persistent graph elements that can be connected to each other via relationships.
///
/// See [`Nodes`](https://redis.io/docs/stack/graph/datatypes/#nodes)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GraphNode {
    pub id: i64,
    pub labels: Vec<String>,
    pub properties: GraphProperties,
}

impl<'de> GraphObjectVisitor<'de> for GraphNode {
    fn visit_seq<A>(mut seq: A, cache: &GraphCache) -> std::result::Result<Self, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let Some(id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(0, &"more elements in sequence"));
        };

        log::debug!("GraphNode::visit_seq, id={id}");

        let Some(label_ids) = seq.next_element::<Vec<usize>>()? else {
            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
        };

        let Some(properties) = seq.next_element_seed(GraphProperties::into_seed(cache))? else {
            return Err(de::Error::invalid_length(2, &"more elements in sequence"));
        };

        let labels = label_ids
            .into_iter()
            .map(|idx| cache.node_labels[idx].clone())
            .collect();

        Ok(GraphNode {
            id,
            labels,
            properties,
        })
    }
}

/// Edges (or Relationships) are persistent graph elements that connect one node to another.
///
/// See [`Relationships`](https://redis.io/docs/stack/graph/datatypes/#relationships)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GraphEdge {
    pub id: i64,
    pub relationship_type: String,
    pub src_node_id: i64,
    pub dst_node_id: i64,
    pub properties: GraphProperties,
}

impl<'de> GraphObjectVisitor<'de> for GraphEdge {
    fn visit_seq<A>(mut seq: A, cache: &GraphCache) -> std::result::Result<Self, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let Some(id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(0, &"more elements in sequence"));
        };

        let Some(rel_type_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
        };

        let Some(src_node_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(2, &"more elements in sequence"));
        };

        let Some(dst_node_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(3, &"more elements in sequence"));
        };

        let Some(properties) = seq.next_element_seed(GraphProperties::into_seed(cache))? else {
            return Err(de::Error::invalid_length(4, &"more elements in sequence"));
        };

        let relationship_type = cache.relationship_types[rel_type_id as usize].clone();

        Ok(GraphEdge {
            id,
            relationship_type,
            src_node_id,
            dst_node_id,
            properties,
        })
    }
}

/// Paths are alternating sequences of nodes and edges, starting and ending with a node.
///
/// See [`Paths`](https://redis.io/docs/stack/graph/datatypes/#paths)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GraphPath {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
impl<'de> GraphObjectVisitor<'de> for GraphPath {
    fn visit_seq<A>(mut seq: A, cache: &GraphCache) -> std::result::Result<Self, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let Some(nodes) =
            seq.next_element_seed(GraphNode::into_vec_seed(cache, GraphValueType::Node))?
        else {
            return Err(de::Error::invalid_length(0, &"more elements in sequence"));
        };

        let Some(edges) =
            seq.next_element_seed(GraphEdge::into_vec_seed(cache, GraphValueType::Edge))?
        else {
            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
        };

        Ok(GraphPath { nodes, edges })
    }
}

/// Properties for a [`Node`](GraphNode) or an [`Edge`](GraphEdge)
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GraphProperties {
    pub properties: HashMap<String, GraphValue>,
}

impl GraphProperties {
    pub fn get_value<T: FromGraphValue>(&self, property_key: &str) -> Result<Option<T>> {
        self.properties
            .get(property_key)
            .map(|v| T::from_graph_value(v.clone()))
            .transpose()
    }
}

impl<'de> GraphObjectVisitor<'de> for (String, GraphValue) {
    fn visit_seq<A>(mut seq: A, cache: &GraphCache) -> std::result::Result<Self, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let Some(property_key_id) = seq.next_element::<usize>()? else {
            return Err(de::Error::invalid_length(0, &"more elements in sequence"));
        };

        let property_key = cache.property_keys[property_key_id].clone();

        let Some(value_type) = seq.next_element::<GraphValueType>()? else {
            return Err(de::Error::invalid_length(1, &"more elements in sequence"));
        };

        let Some(value) = seq.next_element_seed(GraphValueSeed::new(value_type, cache))? else {
            return Err(de::Error::invalid_length(2, &"more elements in sequence"));
        };

        Ok((property_key, value))
    }
}

impl<'de> GraphObjectVisitor<'de> for GraphProperties {
    fn visit_seq<A>(mut seq: A, cache: &GraphCache) -> std::result::Result<Self, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut properties = if let Some(size_hint) = seq.size_hint() {
            HashMap::with_capacity(size_hint)
        } else {
            HashMap::new()
        };

        while let Some((property_key, value)) =
            seq.next_element_seed(<(String, GraphValue)>::into_seed(cache))?
        {
            properties.insert(property_key, value);
        }

        Ok(GraphProperties { properties })
    }
}

/// Used to do [`GraphValue`](GraphValue) to user type conversion
///  while consuming the input [`GraphValue`](GraphValue)
pub trait FromGraphValue: Sized {
    /// Converts to this type from the input [`GraphValue`](GraphValue).
    ///
    /// # Errors
    ///
    /// Any parsing error ([`Error::Client`](crate::Error::Client))
    /// due to incompatibility between Value variant and taget type
    fn from_graph_value(value: GraphValue) -> Result<Self>;
}

impl<T> FromGraphValue for Option<T>
where
    T: FromGraphValue,
{
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Null => Ok(None),
            _ => T::from_graph_value(value).map(|v| Some(v)),
        }
    }
}

impl FromGraphValue for String {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::String(s) => String::from_utf8(s).map_err(|e| Error::Client(e.to_string())),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to String",
                value
            ))),
        }
    }
}

impl FromGraphValue for i64 {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Integer(i) => Ok(i),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to i64",
                value
            ))),
        }
    }
}

impl FromGraphValue for bool {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Boolean(b) => Ok(b),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to bool",
                value
            ))),
        }
    }
}

impl FromGraphValue for f64 {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Double(f) => Ok(f),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to f64",
                value
            ))),
        }
    }
}

impl<T> FromGraphValue for Vec<T>
where
    T: FromGraphValue,
{
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Array(v) => v.into_iter().map(|v| T::from_graph_value(v)).collect(),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to Vec",
                value
            ))),
        }
    }
}

impl FromGraphValue for GraphNode {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Node(n) => Ok(n),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to GraphNode",
                value
            ))),
        }
    }
}

impl FromGraphValue for GraphEdge {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Edge(e) => Ok(e),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to GraphEdge",
                value
            ))),
        }
    }
}

impl FromGraphValue for GraphPath {
    fn from_graph_value(value: GraphValue) -> Result<Self> {
        match value {
            GraphValue::Path(p) => Ok(p),
            _ => Err(Error::Client(format!(
                "Cannot parse result {:?} to GraphPath",
                value
            ))),
        }
    }
}
