use crate::{commands::GraphValueType, resp::Value};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserializer, Deserialize
};
use std::{fmt, marker::PhantomData};

#[derive(Debug, Default)]
pub(crate) struct GraphCache {
    pub node_labels: Vec<String>,
    pub property_keys: Vec<String>,
    pub relationship_types: Vec<String>,
}

impl GraphCache {
    pub fn update(
        &mut self,
        num_node_labels: usize,
        num_prop_keys: usize,
        num_rel_types: usize,
        node_labels: Vec<String>,
        property_keys: Vec<String>,
        relationship_types: Vec<String>,
    ) {
        if self.node_labels.len() == num_node_labels {
            self.node_labels.extend(node_labels);
        } else if self.node_labels.len() < num_node_labels + node_labels.len() {
            self.node_labels
                .extend(node_labels[self.node_labels.len() - num_node_labels..].to_vec());
        }

        if self.property_keys.len() == num_prop_keys {
            self.property_keys.extend(property_keys);
        } else if self.property_keys.len() < num_prop_keys + property_keys.len() {
            self.property_keys
                .extend(property_keys[self.property_keys.len() - num_prop_keys..].to_vec());
        }

        if self.relationship_types.len() == num_rel_types {
            self.relationship_types.extend(relationship_types);
        } else if self.relationship_types.len() < num_rel_types + relationship_types.len() {
            self.relationship_types.extend(
                relationship_types[self.relationship_types.len() - num_rel_types..].to_vec(),
            );
        }
    }


    // returns true if we can parse this result without any cache miss
    pub fn check_for_result<'de, D: Deserializer<'de>>(&self, result: D) -> Result<bool, D::Error> {
        CheckCacheForResultSetSeed::new(&self).deserialize(result)
    }

}

macro_rules! impl_deserialize_seq_for_seed {
    ($struct_name:ident) => {
        impl<'de, 'a> DeserializeSeed<'de> for $struct_name<'a> {
            type Value = bool;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_seq(self)
            }
        }
    };
}

trait CheckCacheFactory<'a> {
    fn new(cache: &'a GraphCache) -> Self;
}

macro_rules! impl_check_cache_factory {
    ($struct_name:ident) => {
        impl<'a> CheckCacheFactory<'a> for $struct_name<'a> {
            fn new(cache: &'a GraphCache) -> Self {
                Self {
                    cache,
                }
            }
        }
    };
}

struct CheckCacheIteratorSeed<'a, ItemSeed> {
    phantom: PhantomData<ItemSeed>,
    cache: &'a GraphCache
}

impl<'a, ItemSeed> CheckCacheFactory<'a> for CheckCacheIteratorSeed<'a, ItemSeed> {
    fn new(cache: &'a GraphCache) -> Self {
        Self {
            phantom: PhantomData,
            cache,
        }
    }
}

impl<'de, 'a, ItemSeed> Visitor<'de> for CheckCacheIteratorSeed<'a, ItemSeed>
where
    ItemSeed: DeserializeSeed<'de, Value = bool> + CheckCacheFactory<'a>,
{
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while let Some(check_item) = seq.next_element_seed(ItemSeed::new(self.cache))? {
            if !check_item {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl<'de, 'a, ItemSeed> DeserializeSeed<'de> for CheckCacheIteratorSeed<'a, ItemSeed>
where
    ItemSeed: DeserializeSeed<'de, Value = bool> + CheckCacheFactory<'a>,
{
    type Value = bool;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct CheckCacheForResultSetSeed<'a> {
    cache: &'a GraphCache
}

impl_check_cache_factory!(CheckCacheForResultSetSeed);
impl_deserialize_seq_for_seed!(CheckCacheForResultSetSeed);

impl<'de, 'a> Visitor<'de> for CheckCacheForResultSetSeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        // some queries don't return results, thus no cache is required
        if let Some(1) = seq.size_hint() {
            return Ok(true);
        }

        let Some(_header) = seq.next_element::<Value>()? else {
            return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
        };

        let Some(check_rows) = 
            seq.next_element_seed(CheckCacheIteratorSeed::<CheckCacheIteratorSeed::<CheckCacheForValueSeed>>::new(self.cache))? else {
            return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
        };

        Ok(check_rows)
    }
}

struct CheckCacheForValueSeed<'a> {
    value_type: GraphValueType,
    cache: &'a GraphCache,
}

impl<'a> CheckCacheFactory<'a> for CheckCacheForValueSeed<'a> {
    #[inline]
    fn new(cache: &'a GraphCache) -> Self {
        Self {
            cache,
            value_type: GraphValueType::Unknown,
        }
    }
}

impl<'a> CheckCacheForValueSeed<'a> {
    #[inline]
    fn with_value_type(value_type: GraphValueType, cache: &'a GraphCache) -> Self {
        Self {
            value_type,
            cache,
        }
    }
}

impl<'de, 'a> Visitor<'de> for CheckCacheForValueSeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let Some(value_type) = seq.next_element::<GraphValueType>()? else {
            return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
        };

        let Some(check_value) = seq.next_element_seed(CheckCacheForValueSeed::with_value_type(value_type, self.cache))? else {
            return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
        };

        Ok(check_value)
    }
}

impl<'de, 'a> DeserializeSeed<'de> for CheckCacheForValueSeed<'a> {
    type Value = bool;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        match self.value_type {
            GraphValueType::Unknown => {
                deserializer.deserialize_seq(self)
            }
            GraphValueType::Null => Ok(true),
            GraphValueType::String => {
                let _string = <&str>::deserialize(deserializer)?;
                Ok(true)
            },
            GraphValueType::Integer => {
                let _integer = i64::deserialize(deserializer)?;
                Ok(true)
            },
            GraphValueType::Boolean => {
                let _boolean = bool::deserialize(deserializer)?;
                Ok(true)
            },
            GraphValueType::Double => {
                let _double = f64::deserialize(deserializer)?;
                Ok(true)
            },
            GraphValueType::Array => CheckCacheIteratorSeed::<CheckCacheForValueSeed>::new(self.cache).deserialize(deserializer),
            GraphValueType::Map => CheckCacheForMapSeed::new(self.cache).deserialize(deserializer),
            GraphValueType::Edge => CheckCacheForEdgeSeed::new(self.cache).deserialize(deserializer),
            GraphValueType::Node => CheckCacheForNodeSeed::new(self.cache).deserialize(deserializer),
            GraphValueType::Path => CheckCacheForPathSeed::new(self.cache).deserialize(deserializer),
            GraphValueType::Point => {
                let _point = <(f32, f32)>::deserialize(deserializer)?;
                Ok(true)
            },
        }
    }
}

struct CheckCacheForMapSeed<'a> {
    cache: &'a GraphCache
}

impl_check_cache_factory!(CheckCacheForMapSeed);
impl_deserialize_seq_for_seed!(CheckCacheForMapSeed);

impl<'de, 'a> Visitor<'de> for CheckCacheForMapSeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while let Some(_key) = seq.next_element::<&str>()? {
            let Some(check_value) = seq.next_element_seed(CheckCacheForValueSeed::new(self.cache))? else {
                return Err(de::Error::custom(&"Cannot parse GraphValue::Map value"));
            };

            if !check_value {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

struct CheckCacheForNodeSeed<'a> {
    cache: &'a GraphCache
}

impl_check_cache_factory!(CheckCacheForNodeSeed);
impl_deserialize_seq_for_seed!(CheckCacheForNodeSeed);

impl<'de, 'a> Visitor<'de> for CheckCacheForNodeSeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let Some(_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
        };

        let Some(label_ids) = seq.next_element::<Vec<usize>>()? else {
            return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
        };

        let num_labels = self.cache.node_labels.len();
        if label_ids.iter().any(|id| *id > num_labels) {
            return Ok(false);
        }

        let Some(check_properties) = seq.next_element_seed(CheckCacheIteratorSeed::<CheckCacheForPropertySeed>::new(self.cache))? else {
            return Err(de::Error::invalid_length(2, &"fewer elements in sequence"));
        };

        Ok(check_properties)
    }
}

struct CheckCacheForEdgeSeed<'a> {
    cache: &'a GraphCache
}

impl_check_cache_factory!(CheckCacheForEdgeSeed);
impl_deserialize_seq_for_seed!(CheckCacheForEdgeSeed);

impl<'de, 'a> Visitor<'de> for CheckCacheForEdgeSeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let Some(_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
        };

        let Some(rel_type_id) = seq.next_element::<usize>()? else {
            return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
        };

        if rel_type_id >= self.cache.relationship_types.len() {
            return Ok(false);
        }

        let Some(_src_node_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(2, &"fewer elements in sequence"));
        };

        let Some(_dst_node_id) = seq.next_element::<i64>()? else {
            return Err(de::Error::invalid_length(3, &"fewer elements in sequence"));
        };

        let Some(check_properties) = seq.next_element_seed(CheckCacheIteratorSeed::<CheckCacheForPropertySeed>::new(self.cache))? else {
            return Err(de::Error::invalid_length(4, &"fewer elements in sequence"));
        };

        Ok(check_properties)
    }
}

struct CheckCacheForPathSeed<'a> {
    cache: &'a GraphCache
}

impl_check_cache_factory!(CheckCacheForPathSeed);
impl_deserialize_seq_for_seed!(CheckCacheForPathSeed);

impl<'de, 'a> Visitor<'de> for CheckCacheForPathSeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let Some(check_nodes) = seq.next_element_seed(CheckCacheForValueSeed::new(self.cache))? else {
            return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
        };

        if !check_nodes {
            return Ok(false);
        }

        let Some(check_edges) = seq.next_element_seed(CheckCacheForValueSeed::new(self.cache))? else {
            return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
        };

        Ok(check_edges)
    }
}

struct CheckCacheForPropertySeed<'a> {
    cache: &'a GraphCache
}

impl_check_cache_factory!(CheckCacheForPropertySeed);
impl_deserialize_seq_for_seed!(CheckCacheForPropertySeed);

impl<'de, 'a> Visitor<'de> for CheckCacheForPropertySeed<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bool")
    }

    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let Some(property_key_id) = seq.next_element::<usize>()? else {
            return Err(de::Error::invalid_length(0, &"fewer elements in sequence"));
        };

        if property_key_id >= self.cache.property_keys.len() {
            return Ok(false);
        }

        let Some(value_type) = seq.next_element::<GraphValueType>()? else {
            return Err(de::Error::invalid_length(1, &"fewer elements in sequence"));
        };

        let Some(check_value) = seq.next_element_seed(CheckCacheForValueSeed::with_value_type(value_type, self.cache))? else {
            return Err(de::Error::invalid_length(2, &"fewer elements in sequence"));
        };

        Ok(check_value)
    }
}

#[cfg(test)]
mod tests {
    use crate::commands::GraphCache;

    #[test]
    fn partial_update() {
        let mut cache = GraphCache {
            node_labels: vec!["node1".to_owned(), "node2".to_owned()],
            property_keys: vec!["prop1".to_owned(), "prop2".to_owned()],
            relationship_types: vec![],
        };

        cache.update(
            2,
            1,
            0,
            vec!["node3".to_owned(), "node4".to_owned(), "node5".to_owned()],
            vec![
                "prop2".to_owned(),
                "prop3".to_owned(),
                "prop4".to_owned(),
                "prop5".to_owned(),
            ],
            vec!["rel1".to_owned()],
        );

        assert_eq!(
            vec![
                "node1".to_owned(),
                "node2".to_owned(),
                "node3".to_owned(),
                "node4".to_owned(),
                "node5".to_owned()
            ],
            cache.node_labels
        );
        assert_eq!(
            vec![
                "prop1".to_owned(),
                "prop2".to_owned(),
                "prop3".to_owned(),
                "prop4".to_owned(),
                "prop5".to_owned()
            ],
            cache.property_keys
        );
        assert_eq!(vec!["rel1".to_owned()], cache.relationship_types);
    }
}
