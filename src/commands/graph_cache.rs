use crate::{resp::Value, GraphValueType};

#[derive(Debug, Default)]
pub(crate) struct GraphCache {
    pub node_labels: Vec<String>,
    pub property_keys: Vec<String>,
    pub relationship_types: Vec<String>,
}

impl GraphCache {
    pub fn update(
        &mut self,
        node_labels: Vec<String>,
        property_keys: Vec<String>,
        relationship_types: Vec<String>,
    ) {
        self.node_labels.extend(node_labels);
        self.property_keys.extend(property_keys);
        self.relationship_types.extend(relationship_types);
    }

    // returns true if we can parse this result without any cache miss
    pub fn check_for_result(&self, result: &Value) -> bool {
        let Value::Array(Some(result_set)) = result else {
            return false;
        };

        // some queries don't return results, thus no cache is required
        if result_set.len() == 1 {
            return true;
        }

        let Value::Array(Some(rows)) = &result_set[1] else {
            return false;
        };

        // no rows, thus no cache is required
        if rows.is_empty() {
            return true;
        }

        let first_row = &rows[0];

        let Value::Array(Some(values)) = first_row else {
            return false;
        };

        values.iter().all(|v| self.check_for_value(v))
    }

    fn check_for_value(&self, value: &Value) -> bool {
        let Value::Array(Some(value_parts)) = value else {
            return false;
        };

        let Value::Integer(value_type) = value_parts[0] else {
            return false;
        };

        let Ok(value_type) = Value::Integer(value_type).into::<GraphValueType>() else {
            return false;
        };

        let value = &value_parts[1];

        match value_type {
            GraphValueType::Array => self.check_for_array(value),
            GraphValueType::Map => self.check_for_map(value),
            GraphValueType::Edge => self.check_cache_for_edge(value),
            GraphValueType::Node => self.check_for_node(value),
            GraphValueType::Path => self.check_cache_for_path(value),
            _ => true,
        }
    }

    fn check_for_array(&self, value: &Value) -> bool {
        let Value::Array(Some(values)) = value else {
            return false;
        };

        values.iter().all(|v| self.check_for_value(v))
    }

    fn check_for_map(&self, value: &Value) -> bool {
        let Value::Array(Some(values)) = value else {
            return false;
        };

        let mut iter = values.iter();
        while let Some(_key) = iter.next() {
            let Some(value) = iter.next() else {
                return false;
            };

            if ! self.check_for_value(value) {
                return false;
            }
        }

        true
    }

    fn check_for_node(&self, node: &Value) -> bool {
        let Value::Array(Some(node_parts)) = node else {
            return false;
        };

        let Value::Array(Some(node_labels)) = &node_parts[1] else {
            return false;
        };

        for node_label in node_labels {
            let Value::Integer(node_label) = node_label else {
                return false;
            };

            if *node_label >= self.node_labels.len() as i64 {
                return false;
            }
        }

        self.check_for_properties(&node_parts[2])
    }

    fn check_cache_for_edge(&self, edge: &Value) -> bool {
        let Value::Array(Some(edge_parts)) = edge else {
            return false;
        };

        let Value::Integer(rel_type_id) = edge_parts[1] else {
            return false;
        };

        if rel_type_id >= self.relationship_types.len() as i64 {
            return false;
        }

        self.check_for_properties(&edge_parts[4])
    }

    fn check_cache_for_path(&self, path: &Value) -> bool {
        let Value::Array(Some(path_parts)) = path else {
            return false;
        };

        // nodes & edges
        self.check_for_array(&path_parts[0])
            && self.check_for_array(&path_parts[1])
    }

    fn check_for_properties(&self, properties: &Value) -> bool {
        let Value::Array(Some(properties)) = properties else {
            return false;
        };

        for property in properties {
            let Value::Array(Some(property)) = property else {
                return false;
            };

            let Value::Integer(property_key_id) = property[0] else {
                return false;
            };

            if property_key_id >= self.property_keys.len() as i64 {
                return false;
            }
        }

        true
    }
}