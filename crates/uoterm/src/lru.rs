//! A small cache that forgets the entry used least lately: the drawn
//! words of the window, and the encoded tiles of the world map a web page
//! asks for.

use std::collections::HashMap;
use std::hash::Hash;

/// A cache that forgets the entry used least lately when it is full.
pub struct LruCache<K, V> {
    entries: HashMap<K, (V, u64)>,
    capacity: usize,
    clock: u64,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            clock: 0,
        }
    }

    /// The value of `key`, when it is kept.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.clock += 1;
        let entry = self.entries.get_mut(key)?;
        entry.1 = self.clock;
        Some(&entry.0)
    }

    /// Keeps `value` under `key`.
    pub fn insert(&mut self, key: K, value: V) {
        self.clock += 1;
        if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity {
            self.forget_oldest();
        }
        self.entries.insert(key, (value, self.clock));
    }

    /// The value of `key`. `make` gives it the first time.
    pub fn get_or_make(&mut self, key: &K, make: impl FnOnce() -> V) -> &V {
        self.clock += 1;
        if !self.entries.contains_key(key) {
            if self.entries.len() >= self.capacity {
                self.forget_oldest();
            }
            self.entries.insert(key.clone(), (make(), self.clock));
        }
        let entry = self.entries.get_mut(key).expect("the entry was just made");
        entry.1 = self.clock;
        &entry.0
    }

    fn forget_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, (_, used))| *used)
            .map(|(key, _)| key.clone());
        if let Some(key) = oldest {
            self.entries.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cache_forgets_the_entry_used_least_lately() {
        let mut cache = LruCache::new(2);
        let mut made = 0;
        let mut get = |cache: &mut LruCache<u32, u32>, key: u32| {
            *cache.get_or_make(&key, || {
                made += 1;
                key * 10
            })
        };
        assert_eq!(get(&mut cache, 1), 10);
        assert_eq!(get(&mut cache, 2), 20);
        assert_eq!(get(&mut cache, 1), 10);
        assert_eq!(get(&mut cache, 3), 30);
        // Key 2 was used least lately, so it went, and 1 stayed.
        assert_eq!(get(&mut cache, 1), 10);
        assert_eq!(get(&mut cache, 2), 20);
        assert_eq!(made, 4);
    }

    #[test]
    fn a_kept_value_is_read_and_a_new_one_takes_the_oldest_place() {
        let mut cache = LruCache::new(1);
        cache.insert(1, "one");
        assert_eq!(cache.get(&1), Some(&"one"));
        cache.insert(2, "two");
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&"two"));
    }
}
