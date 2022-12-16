use crate::{Error, Result};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
};

/// A struct which goal is to give a generic access to attach any state to a client instance
///
/// It is internally used to cache [RedisGraph](crate::commands::GraphCommands) metadata.
#[derive(Default)]
pub struct ClientState {
    cache: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ClientState {
    pub(crate) fn new() -> ClientState {
        ClientState {
            cache: HashMap::new(),
        }
    }

    /// Get state with a specific type `S` for a specific `key`
    ///
    /// # Return
    /// Casted state to the required type or Ok(None) if `key` has not been found.
    ///
    /// If the state does not already exists, it is created on the fly
    /// by calling `S::default()`
    ///
    /// # Errors
    /// An error if an entry has been found for the `key` but this entry cannot be
    /// downcasted to the required type.
    pub fn get_state<S: Default + Send + Sync + 'static>(&self, key: &str) -> Result<Option<&S>> {
        match self.cache.get(key) {
            Some(cache_entry) => match cache_entry.downcast_ref::<S>() {
                Some(cache_entry) => Ok(Some(cache_entry)),
                None => Err(Error::Client(format!("Cannot downcast cache entry '{key}'"))),
            },
            None => Ok(None),
        }
    }

    /// Get state with a specific type `S` for a specific `key`
    ///
    /// # Return
    /// Casted state to the required type.
    ///
    /// If the state does not already exists, it is created on the fly
    /// by calling `S::default()`
    ///
    /// # Errors
    /// An error if an entry has been found for the `key` but this entry cannot be
    /// downcasted to the required type.
    pub fn get_state_mut<S: Default + Send + Sync + 'static>(
        &mut self,
        key: &str,
    ) -> Result<&mut S> {
        let cache_entry = match self.cache.entry(key.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => v.insert(Box::new(S::default())),
        };

        let cache_entry = cache_entry
            .downcast_mut::<S>()
            .ok_or_else(|| Error::Client(format!("Cannot downcast cache entry '{key}'")));

        cache_entry
    }
}
