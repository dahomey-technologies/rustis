use crate::{Error, Result};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
};

/// A cache which goal is to give a generic access to attach any state to a client instance
/// 
/// It is internally used to cache [RedisGraph](crate::commands::GraphCommands) metadata.
#[derive(Default)]
pub struct Cache {
    cache: HashMap<String, Box<dyn Any + Send>>,
}

impl Cache {
    pub(crate) fn new() -> Cache {
        Cache {
            cache: HashMap::new(),
        }
    }

    /// Get cache entry with a specific type `E` for a specific `key`
    /// 
    /// # Return 
    /// Casted cache entry to the required type.
    /// 
    /// If the cache entry does not already exists, it is created on the fly
    /// by calling `E::default()`
    /// 
    /// # Errors
    /// An error if an entry has been found for the `key` but this entry cannot be
    /// downcasted to the required type.
    pub fn get_entry<E: Default + Send + 'static>(&mut self, key: &str) -> Result<&mut E> {
        let cache_entry = match self.cache.entry(key.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Box::new(E::default())),
        };

        let cache_entry = cache_entry
            .downcast_mut::<E>()
            .ok_or_else(|| Error::Client(format!("Cannot downcast cache entry '{key}'")));

        cache_entry
    }
}
