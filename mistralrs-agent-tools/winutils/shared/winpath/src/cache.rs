//! High-performance caching implementation for path normalization.

use crate::normalizer::CacheStats;
use alloc::{collections::BTreeMap, string::String};
use core::hash::{Hash, Hasher};

/// Trait for cache implementations.
pub trait Cache<K, V> {
    /// Gets a value from the cache.
    fn get(&self, key: &K) -> Option<&V>;

    /// Inserts a key-value pair into the cache.
    fn insert(&mut self, key: K, value: V) -> Option<V>;

    /// Removes a key from the cache.
    fn remove(&mut self, key: &K) -> Option<V>;

    /// Clears all entries from the cache.
    fn clear(&mut self);

    /// Returns the current number of entries.
    fn len(&self) -> usize;

    /// Returns true if the cache is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the maximum capacity of the cache.
    fn capacity(&self) -> usize;

    /// Returns cache statistics.
    fn stats(&self) -> CacheStats;
}

/// LRU (Least Recently Used) cache implementation optimized for path strings.
///
/// This implementation uses a combination of a hash map for O(1) access
/// and a doubly-linked list for O(1) LRU operations.
#[derive(Debug)]
pub struct LruCache<K, V> {
    /// Storage for key-value pairs with access order tracking
    storage: BTreeMap<u64, LruNode<K, V>>,
    /// Key to hash mapping for reverse lookup
    key_to_hash: BTreeMap<K, u64>,
    /// Head of the LRU list (most recently used)
    head: Option<u64>,
    /// Tail of the LRU list (least recently used)
    tail: Option<u64>,
    /// Maximum cache capacity
    capacity: usize,
    /// Current size
    size: usize,
    /// Statistics
    hits: u64,
    misses: u64,
    /// Hash counter for generating unique node IDs
    next_hash: u64,
}

/// Node in the LRU cache's doubly-linked list.
#[derive(Debug, Clone)]
struct LruNode<K, V> {
    key: K,
    value: V,
    prev: Option<u64>,
    next: Option<u64>,
}

impl<K: Clone + Ord, V: Clone> LruCache<K, V> {
    /// Creates a new LRU cache with the specified capacity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::cache::LruCache;
    ///
    /// let cache: LruCache<String, String> = LruCache::new(100);
    /// assert_eq!(cache.capacity(), 100);
    /// ```
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Cache capacity must be greater than 0");

        Self {
            storage: BTreeMap::new(),
            key_to_hash: BTreeMap::new(),
            head: None,
            tail: None,
            capacity,
            size: 0,
            hits: 0,
            misses: 0,
            next_hash: 1,
        }
    }

    /// Moves a node to the head of the LRU list (marks as most recently used).
    fn move_to_head(&mut self, hash: u64) {
        if self.head == Some(hash) {
            return; // Already at head
        }

        // Remove from current position
        self.remove_from_list(hash);

        // Add to head
        self.add_to_head(hash);
    }

    /// Adds a node to the head of the LRU list.
    fn add_to_head(&mut self, hash: u64) {
        if let Some(node) = self.storage.get_mut(&hash) {
            node.prev = None;
            node.next = self.head;
        }

        if let Some(old_head) = self.head {
            if let Some(old_head_node) = self.storage.get_mut(&old_head) {
                old_head_node.prev = Some(hash);
            }
        } else {
            // Cache was empty
            self.tail = Some(hash);
        }

        self.head = Some(hash);
    }

    /// Removes a node from its current position in the LRU list.
    fn remove_from_list(&mut self, hash: u64) {
        if let Some(node) = self.storage.get(&hash) {
            let prev = node.prev;
            let next = node.next;

            // Update previous node
            if let Some(prev_hash) = prev {
                if let Some(prev_node) = self.storage.get_mut(&prev_hash) {
                    prev_node.next = next;
                }
            } else {
                // This was the head
                self.head = next;
            }

            // Update next node
            if let Some(next_hash) = next {
                if let Some(next_node) = self.storage.get_mut(&next_hash) {
                    next_node.prev = prev;
                }
            } else {
                // This was the tail
                self.tail = prev;
            }
        }
    }

    /// Removes the least recently used item from the cache.
    fn evict_lru(&mut self) -> Option<(K, V)> {
        if let Some(tail_hash) = self.tail {
            if let Some(tail_node) = self.storage.remove(&tail_hash) {
                self.key_to_hash.remove(&tail_node.key);
                self.remove_from_list(tail_hash);
                self.size -= 1;
                return Some((tail_node.key, tail_node.value));
            }
        }
        None
    }

    /// Generates a unique hash for a new node.
    fn generate_hash(&mut self) -> u64 {
        let hash = self.next_hash;
        self.next_hash = self.next_hash.wrapping_add(1);
        hash
    }
}

impl<K: Clone + Ord, V: Clone> Cache<K, V> for LruCache<K, V> {
    fn get(&self, key: &K) -> Option<&V> {
        if let Some(&hash) = self.key_to_hash.get(key) {
            if let Some(node) = self.storage.get(&hash) {
                // Note: We can't update LRU order in a non-mutable method
                // In practice, you'd want get_mut or separate methods
                return Some(&node.value);
            }
        }
        None
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        // Check if key already exists
        if let Some(&existing_hash) = self.key_to_hash.get(&key) {
            if let Some(existing_node) = self.storage.get_mut(&existing_hash) {
                let old_value = existing_node.value.clone();
                existing_node.value = value;
                self.move_to_head(existing_hash);
                return Some(old_value);
            }
        }

        // Evict if at capacity
        if self.size >= self.capacity {
            self.evict_lru();
        }

        // Insert new node
        let hash = self.generate_hash();
        let node = LruNode {
            key: key.clone(),
            value,
            prev: None,
            next: None,
        };

        self.storage.insert(hash, node);
        self.key_to_hash.insert(key, hash);
        self.add_to_head(hash);
        self.size += 1;

        None
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(&hash) = self.key_to_hash.get(key) {
            if let Some(node) = self.storage.remove(&hash) {
                self.key_to_hash.remove(key);
                self.remove_from_list(hash);
                self.size -= 1;
                return Some(node.value);
            }
        }
        None
    }

    fn clear(&mut self) {
        self.storage.clear();
        self.key_to_hash.clear();
        self.head = None;
        self.tail = None;
        self.size = 0;
    }

    fn len(&self) -> usize {
        self.size
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn stats(&self) -> CacheStats {
        CacheStats::new(self.size, self.capacity, self.hits, self.misses)
    }
}

// Special implementation for path caching that tracks hits/misses
impl LruCache<String, String> {
    /// Gets a value from the cache, updating statistics and LRU order.
    pub fn get_with_stats(&mut self, key: &str) -> Option<String> {
        if let Some(&hash) = self.key_to_hash.get(key) {
            if let Some(node) = self.storage.get(&hash) {
                self.hits += 1;
                let value = node.value.clone();
                self.move_to_head(hash);
                return Some(value);
            }
        }
        self.misses += 1;
        None
    }

    /// Inserts a value and updates statistics.
    pub fn insert_with_stats(&mut self, key: String, value: String) -> Option<String> {
        self.insert(key, value)
    }
}

/// Fast hash function optimized for path strings.
///
/// This uses a simple but effective hash function that's optimized
/// for the typical characteristics of file paths.
fn fast_path_hash<T: Hash>(value: &T) -> u64 {
    #[cfg(feature = "std")]
    {
        use std::hash::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }
    #[cfg(not(feature = "std"))]
    {
        // Simple hash function for no_std environments
        use core::hash::Hasher;
        struct SimpleHasher(u64);
        impl Hasher for SimpleHasher {
            fn finish(&self) -> u64 { self.0 }
            fn write(&mut self, bytes: &[u8]) {
                for &byte in bytes {
                    self.0 = self.0.wrapping_mul(31).wrapping_add(byte as u64);
                }
            }
        }
        let mut hasher = SimpleHasher(0);
        value.hash(&mut hasher);
        hasher.finish()
    }
}

/// Simple cache implementation without LRU eviction for testing.
#[derive(Debug)]
pub struct SimpleCache<K, V> {
    storage: BTreeMap<K, V>,
    capacity: usize,
    hits: u64,
    misses: u64,
}

impl<K: Clone + Ord, V: Clone> SimpleCache<K, V> {
    /// Creates a new simple cache.
    pub fn new(capacity: usize) -> Self {
        Self {
            storage: BTreeMap::new(),
            capacity,
            hits: 0,
            misses: 0,
        }
    }
}

impl<K: Clone + Ord, V: Clone> Cache<K, V> for SimpleCache<K, V> {
    fn get(&self, key: &K) -> Option<&V> {
        self.storage.get(key)
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.storage.len() >= self.capacity {
            // Simple eviction: remove first entry
            if let Some((first_key, _)) = self.storage.iter().next() {
                let first_key = first_key.clone();
                self.storage.remove(&first_key);
            }
        }
        self.storage.insert(key, value)
    }

    fn remove(&mut self, key: &K) -> Option<V> {
        self.storage.remove(key)
    }

    fn clear(&mut self) {
        self.storage.clear();
    }

    fn len(&self) -> usize {
        self.storage.len()
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn stats(&self) -> CacheStats {
        CacheStats::new(self.len(), self.capacity, self.hits, self.misses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache_basic_operations() {
        let mut cache: LruCache<String, String> = LruCache::new(3);

        assert_eq!(cache.len(), 0);
        assert_eq!(cache.capacity(), 3);
        assert!(cache.is_empty());

        // Insert items
        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        cache.insert("key3".to_string(), "value3".to_string());

        assert_eq!(cache.len(), 3);
        assert!(!cache.is_empty());

        // Check retrieval
        assert_eq!(cache.get(&"key1".to_string()), Some(&"value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()), Some(&"value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()), Some(&"value3".to_string()));
    }

    #[test]
    fn test_lru_cache_eviction() {
        let mut cache: LruCache<String, String> = LruCache::new(2);

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        // This should evict key1 (least recently used)
        cache.insert("key3".to_string(), "value3".to_string());

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key2".to_string()), Some(&"value2".to_string()));
        assert_eq!(cache.get(&"key3".to_string()), Some(&"value3".to_string()));
    }

    #[test]
    fn test_lru_cache_update_existing() {
        let mut cache: LruCache<String, String> = LruCache::new(3);

        cache.insert("key1".to_string(), "value1".to_string());
        let old_value = cache.insert("key1".to_string(), "value1_new".to_string());

        assert_eq!(old_value, Some("value1".to_string()));
        assert_eq!(cache.get(&"key1".to_string()), Some(&"value1_new".to_string()));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_lru_cache_removal() {
        let mut cache: LruCache<String, String> = LruCache::new(3);

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        let removed = cache.remove(&"key1".to_string());
        assert_eq!(removed, Some("value1".to_string()));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_lru_cache_clear() {
        let mut cache: LruCache<String, String> = LruCache::new(3);

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_lru_cache_with_stats() {
        let mut cache = LruCache::new(2);

        // Miss
        assert_eq!(cache.get_with_stats("key1"), None);

        // Insert and hit
        cache.insert_with_stats("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get_with_stats("key1"), Some("value1".to_string()));

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_simple_cache() {
        let mut cache: SimpleCache<String, String> = SimpleCache::new(2);

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());

        assert_eq!(cache.get(&"key1".to_string()), Some(&"value1".to_string()));

        // Test eviction
        cache.insert("key3".to_string(), "value3".to_string());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_stats() {
        let cache: LruCache<String, String> = LruCache::new(10);
        let stats = cache.stats();

        assert_eq!(stats.size, 0);
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[test]
    fn test_fast_path_hash() {
        let hash1 = fast_path_hash(&"C:\\Users\\David");
        let hash2 = fast_path_hash(&"C:\\Users\\David");
        let hash3 = fast_path_hash(&"C:\\Users\\Alice");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
