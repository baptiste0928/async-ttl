use std::{
    collections::{BTreeMap, HashMap},
    hash::Hash,
};

/// Map abstraction for [`AsyncTtl`].
///
/// This trait is used to allow various types of maps to be used in [`AsyncTtl`]
/// including custom ones. The trait is implemented by default for [`HashMap`]
/// and [`BTreeMap`].
///
/// Methods are suffixed with `_cache` to differentiate them from default
/// type methods.
///
/// [`AsyncTtl`]: crate::AsyncTtl
pub trait CacheMap<K, V> {
    /// Insert a new entry in the map.
    fn insert_cache(&mut self, key: K, value: V);

    /// Remove an entry from the map.
    fn remove_cache(&mut self, key: &K);
}

impl<K, V> CacheMap<K, V> for HashMap<K, V>
where
    K: Hash + Eq,
{
    fn insert_cache(&mut self, key: K, value: V) {
        self.insert(key, value);
    }

    fn remove_cache(&mut self, key: &K) {
        self.remove(key);
    }
}

impl<K, V> CacheMap<K, V> for BTreeMap<K, V>
where
    K: Ord,
{
    fn insert_cache(&mut self, key: K, value: V) {
        self.insert(key, value);
    }

    fn remove_cache(&mut self, key: &K) {
        self.remove(key);
    }
}
