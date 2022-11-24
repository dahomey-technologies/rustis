use crate::{Error, Result};
use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
};

#[derive(Default)]
pub struct Cache {
    cache: HashMap<String, Box<dyn Any + Send>>,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            cache: HashMap::new(),
        }
    }

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
