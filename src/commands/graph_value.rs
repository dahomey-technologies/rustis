use crate::{
    commands::GraphCache,
    resp::{FromValue, Value, BulkString},
    Error, Result,
};
use std::collections::HashMap;

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

impl FromValue for GraphValueType {
    fn from_value(value: Value) -> Result<Self> {
        let value_type: u8 = value.into()?;

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
            _ => Err(Error::Client(format!(
                "Cannot parse GraphValueType for value type '{value_type}'"
            ))),
        }
    }
}

/// Object model for the different [`RedisGraph Data Types`](https://redis.io/docs/stack/graph/datatypes/)
#[derive(Debug, Clone, PartialEq)]
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

    pub(crate) fn from_value(value: Value, cache: &GraphCache) -> Result<Self> {
        let (value_type, value): (GraphValueType, Value) = value.into()?;
        Self::from_type_and_value(value_type, value, cache)
    }

    pub(crate) fn from_type_and_value(value_type: GraphValueType, value: Value, cache: &GraphCache) -> Result<Self> {
        let graph_value = match value_type {
            GraphValueType::Unknown => {
                return Err(Error::Client(
                    "Unknown value type is not supported".to_owned(),
                ))
            }
            GraphValueType::Null => GraphValue::Null,
            GraphValueType::String => GraphValue::String(value.into::<BulkString>()?.0),
            GraphValueType::Integer => GraphValue::Integer(value.into()?),
            GraphValueType::Boolean => GraphValue::Boolean(value.into()?),
            GraphValueType::Double => GraphValue::Double(value.into()?),
            GraphValueType::Array => {
                let Value::Array(values) = value else {
                    return Err(Error::Client("Cannot parse GraphValue".to_owned()));
                };

                let array = values
                    .into_iter()
                    .map(|v| GraphValue::from_value(v, cache))
                    .collect::<Result<Vec<GraphValue>>>()?;

                GraphValue::Array(array)
            }
            GraphValueType::Edge => GraphValue::Edge(GraphEdge::from_value(value, cache)?),
            GraphValueType::Node => GraphValue::Node(GraphNode::from_value(value, cache)?),
            GraphValueType::Path => GraphValue::Path(GraphPath::from_value(value, cache)?),
            GraphValueType::Map => {
                let Value::Array(values) = value else {
                    return Err(Error::Client("Cannot parse GraphValue".to_owned()));
                };

                let mut map: HashMap<String, GraphValue> = HashMap::with_capacity(values.len() / 2);
                let mut iter = values.into_iter();
                while let Some(key) = iter.next() {
                    let Some(value) = iter.next() else {
                        return Err(Error::Client("Cannot parse GraphValue".to_owned()));
                    };

                    map.insert(key.into()?, Self::from_value(value, cache)?);
                }

                GraphValue::Map(map)
            }
            GraphValueType::Point => GraphValue::Point(value.into()?),
        };

        Ok(graph_value)
    }
}

/// Nodes are persistent graph elements that can be connected to each other via relationships.
/// 
/// See [`Nodes`](https://redis.io/docs/stack/graph/datatypes/#nodes)
#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
    pub id: i64,
    pub labels: Vec<String>,
    pub properties: GraphProperties,
}

impl GraphNode {
    pub(crate) fn from_value(value: Value, cache: &GraphCache) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut iter = values.into_iter();

        let (Some(Value::Integer(id)), Some(labels), Some(properties)) 
            = (iter.next(), iter.next(), iter.next()) else {
            return Err(Error::Client("Cannot parse GraphNode".to_owned()));         
        };

        let labels = labels.into::<Vec<usize>>()?
            .into_iter()
            .map(|idx| cache.node_labels[idx].clone())
            .collect();

        let properties = GraphProperties::from_value(properties, cache)?;

        Ok(Self {
            id,
            labels,
            properties,
        })
    }
}

/// Edges (or Relationships) are persistent graph elements that connect one node to another.
/// 
/// See [`Relationships`](https://redis.io/docs/stack/graph/datatypes/#relationships)
#[derive(Debug, Clone, PartialEq)]
pub struct GraphEdge {
    pub id: i64,
    pub relationship_type: String,
    pub src_node_id: i64,
    pub dst_node_id: i64,
    pub properties: GraphProperties,
}

impl GraphEdge {
    pub(crate) fn from_value(value: Value, cache: &GraphCache) -> Result<Self> {
        let values: Vec<Value> = value.into()?;
        let mut iter = values.into_iter();

        let (
            Some(Value::Integer(id)), 
            Some(Value::Integer(rel_type_id)), 
            Some(Value::Integer(src_node_id)), 
            Some(Value::Integer(dst_node_id)), 
            Some(properties)
        ) = (iter.next(), iter.next(), iter.next(), iter.next(), iter.next()) else {
            return Err(Error::Client("Cannot parse GraphNode".to_owned()));         
        };

        let relationship_type = cache.relationship_types[rel_type_id as usize].clone();

        let properties = GraphProperties::from_value(properties, cache)?;

        Ok(Self {
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
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPath {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl GraphPath {
    pub(crate) fn from_value(value: Value, cache: &GraphCache) -> Result<Self> {
        let (nodes, edges): (Value, Value) = value.into()?;

        let nodes: Vec<GraphNode> = GraphValue::from_value(nodes, cache)?.into()?;
        let edges: Vec<GraphEdge> = GraphValue::from_value(edges, cache)?.into()?;

        Ok(Self {
            nodes,
            edges
        })
    }
}

/// Properties for a [`Node`](GraphNode) or an [`Edge`](GraphEdge)
#[derive(Debug, Clone, PartialEq)]
pub struct GraphProperties {
    pub properties: HashMap<String, GraphValue>,
}

impl GraphProperties {
    pub(crate) fn from_value(value: Value, cache: &GraphCache) -> Result<Self> {
        let values: Vec<Value> = value.into()?;

        let properties = values
            .into_iter()
            .map(|v| {
                let (property_key_id, value_type, value): (usize, GraphValueType, Value) = v.into()?;
                let property_key = cache.property_keys[property_key_id].clone();
                let value = GraphValue::from_type_and_value(value_type, value, cache)?;

                Ok((property_key, value))
            })
            .collect::<Result<HashMap<String, GraphValue>>>()?;

        Ok(Self {
            properties,
        })
    }

    pub fn get_value<T: FromGraphValue>(&self, property_key: &str) -> Result<Option<T>> {
        self.properties.get(property_key).map(|v| T::from_graph_value(v.clone())).transpose()
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
