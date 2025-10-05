//! High-performance multi-level cache with lock-free operations and Bloom filters.

use crate::{NormalizationResult, PathError, Result};
use ahash::{AHasher, AHashMap};
use bloomfilter::Bloom;
use compact_str::CompactString;
use dashmap::DashMap;
use lru::LruCache;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use smallvec::SmallVec;
use std::alloc::{alloc, dealloc, Layout};
use std::borrow::Cow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::ptr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub l1_hits: AtomicU64,
    pub l1_misses: AtomicU64,
    pub l2_hits: AtomicU64,
    pub l2_misses: AtomicU64,
    pub bloom_hits: AtomicU64,
    pub bloom_false_positives: AtomicU64,
    pub total_lookups: AtomicU64,
    pub total_insertions: AtomicU64,
    pub evictions: AtomicU64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.total_lookups.load(Ordering::Relaxed) as f64;
        if total == 0.0 {
            return 0.0;
        }
        let hits = (self.l1_hits.load(Ordering::Relaxed) + self.l2_hits.load(Ordering::Relaxed)) as f64;
        hits / total * 100.0
    }

    pub fn l1_hit_rate(&self) -> f64 {
        let total = self.total_lookups.load(Ordering::Relaxed) as f64;
        if total == 0.0 {
            return 0.0;
        }
        self.l1_hits.load(Ordering::Relaxed) as f64 / total * 100.0
    }
}

// Thread-local L1 cache for hot paths
thread_local! {
    static L1_CACHE: RefCell<LruCache<CompactString, Arc<CachedPath>>> = {
        RefCell::new(LruCache::new(std::num::NonZeroUsize::new(64).unwrap()))
    };
}

// Cached path entry with metadata
#[derive(Debug, Clone)]
struct CachedPath {
    normalized: CompactString,
    original_format: u8,
    has_long_prefix: bool,
    last_accessed: Instant,
}

/// Multi-level path cache with advanced features
pub struct PathCache {
    // L2 shared cache (lock-free)
    l2_cache: Arc<DashMap<CompactString, Arc<CachedPath>, ahash::RandomState>>,

    // Bloom filter for negative cache (paths that don't exist)
    bloom_filter: Arc<RwLock<Bloom<CompactString>>>,

    // Pre-warmed common paths
    prewarmed: Arc<DashMap<CompactString, Arc<CachedPath>, ahash::RandomState>>,

    // Statistics
    stats: Arc<CacheStats>,

    // Configuration
    config: CacheConfig,

    // Memory pool for string allocations
    string_pool: Arc<StringPool>,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_size: usize,
    pub l2_size: usize,
    pub bloom_size: usize,
    pub bloom_fp_rate: f64,
    pub ttl: Option<Duration>,
    pub enable_stats: bool,
    pub prewarm_paths: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 64,
            l2_size: 4096,
            bloom_size: 100_000,
            bloom_fp_rate: 0.01,
            ttl: Some(Duration::from_secs(300)),
            enable_stats: true,
            prewarm_paths: vec![
                "C:\\Windows".into(),
                "C:\\Windows\\System32".into(),
                "C:\\Program Files".into(),
                "C:\\Program Files (x86)".into(),
                "C:\\Users".into(),
                "/mnt/c".into(),
                "/mnt/c/Windows".into(),
                "/mnt/c/Users".into(),
            ],
        }
    }
}

impl PathCache {
    /// Create a new multi-level cache
    pub fn new(config: CacheConfig) -> Self {
        let l2_cache = Arc::new(DashMap::with_capacity_and_hasher(
            config.l2_size,
            ahash::RandomState::new(),
        ));

        let bloom_filter = Arc::new(RwLock::new(Bloom::new_for_fp_rate(
            config.bloom_size,
            config.bloom_fp_rate,
        )));

        let prewarmed = Arc::new(DashMap::with_hasher(ahash::RandomState::new()));

        // Pre-warm cache with common paths
        for path in &config.prewarm_paths {
            if let Ok(normalized) = crate::normalize_path(path) {
                let cached = Arc::new(CachedPath {
                    normalized: CompactString::from(normalized),
                    original_format: 0,
                    has_long_prefix: false,
                    last_accessed: Instant::now(),
                });
                prewarmed.insert(CompactString::from(path), cached);
            }
        }

        Self {
            l2_cache,
            bloom_filter,
            prewarmed,
            stats: Arc::new(CacheStats::default()),
            config,
            string_pool: Arc::new(StringPool::new(1024 * 1024)), // 1MB pool
        }
    }

    /// Look up a path in the cache
    pub fn get(&self, path: &str) -> Option<String> {
        if self.config.enable_stats {
            self.stats.total_lookups.fetch_add(1, Ordering::Relaxed);
        }

        let key = CompactString::from(path);

        // Check L1 cache first (thread-local)
        if let Some(cached) = L1_CACHE.with(|cache| {
            cache.borrow_mut().get(&key).cloned()
        }) {
            if self.config.enable_stats {
                self.stats.l1_hits.fetch_add(1, Ordering::Relaxed);
            }
            return Some(cached.normalized.to_string());
        }

        // Check pre-warmed paths
        if let Some(cached) = self.prewarmed.get(&key) {
            if self.config.enable_stats {
                self.stats.l1_misses.fetch_add(1, Ordering::Relaxed);
                self.stats.l2_hits.fetch_add(1, Ordering::Relaxed);
            }

            // Promote to L1
            let cached = cached.clone();
            L1_CACHE.with(|cache| {
                cache.borrow_mut().put(key.clone(), cached.clone());
            });

            return Some(cached.normalized.to_string());
        }

        // Check L2 cache
        if let Some(mut entry) = self.l2_cache.get_mut(&key) {
            // Check TTL if configured
            if let Some(ttl) = self.config.ttl {
                if entry.last_accessed.elapsed() > ttl {
                    // Entry expired, remove it
                    drop(entry);
                    self.l2_cache.remove(&key);
                    if self.config.enable_stats {
                        self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                    }
                    return None;
                }
            }

            if self.config.enable_stats {
                self.stats.l1_misses.fetch_add(1, Ordering::Relaxed);
                self.stats.l2_hits.fetch_add(1, Ordering::Relaxed);
            }

            // Update last accessed time
            let mut cached = (**entry).clone();
            cached.last_accessed = Instant::now();
            let cached = Arc::new(cached);
            *entry = cached.clone();

            // Promote to L1
            L1_CACHE.with(|cache| {
                cache.borrow_mut().put(key.clone(), cached.clone());
            });

            return Some(cached.normalized.to_string());
        }

        // Check Bloom filter for negative cache
        if self.bloom_filter.read().check(&key) {
            if self.config.enable_stats {
                self.stats.bloom_hits.fetch_add(1, Ordering::Relaxed);
            }
            // Might be a false positive, but likely doesn't exist
            return None;
        }

        if self.config.enable_stats {
            self.stats.l1_misses.fetch_add(1, Ordering::Relaxed);
            self.stats.l2_misses.fetch_add(1, Ordering::Relaxed);
        }

        None
    }

    /// Insert a normalized path into the cache
    pub fn insert(&self, original: &str, normalized: String, format: u8, has_long_prefix: bool) {
        if self.config.enable_stats {
            self.stats.total_insertions.fetch_add(1, Ordering::Relaxed);
        }

        let key = CompactString::from(original);
        let cached = Arc::new(CachedPath {
            normalized: CompactString::from(normalized),
            original_format: format,
            has_long_prefix,
            last_accessed: Instant::now(),
        });

        // Insert into L1
        L1_CACHE.with(|cache| {
            cache.borrow_mut().put(key.clone(), cached.clone());
        });

        // Insert into L2
        if self.l2_cache.len() >= self.config.l2_size {
            // Simple eviction: remove oldest entry
            // In production, use more sophisticated eviction like LFU
            if let Some(oldest) = self.l2_cache.iter().min_by_key(|entry| entry.last_accessed) {
                let oldest_key = oldest.key().clone();
                self.l2_cache.remove(&oldest_key);
                if self.config.enable_stats {
                    self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        self.l2_cache.insert(key, cached);
    }

    /// Add a path to the negative cache (Bloom filter)
    pub fn add_negative(&self, path: &str) {
        let key = CompactString::from(path);
        self.bloom_filter.write().set(&key);
    }

    /// Clear all caches
    pub fn clear(&self) {
        L1_CACHE.with(|cache| cache.borrow_mut().clear());
        self.l2_cache.clear();
        *self.bloom_filter.write() = Bloom::new_for_fp_rate(
            self.config.bloom_size,
            self.config.bloom_fp_rate,
        );
    }

    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Warm the cache with a batch of paths
    pub async fn warm_batch(&self, paths: &[&str]) {
        use rayon::prelude::*;

        paths.par_iter().for_each(|path| {
            if let Ok(normalized) = crate::normalize_path(path) {
                self.insert(path, normalized, 0, false);
            }
        });
    }
}

/// Memory pool for string allocations
struct StringPool {
    pool: Mutex<Vec<Box<[u8; 256]>>>,
    allocated: AtomicUsize,
    max_size: usize,
}

impl StringPool {
    fn new(max_size: usize) -> Self {
        Self {
            pool: Mutex::new(Vec::with_capacity(max_size / 256)),
            allocated: AtomicUsize::new(0),
            max_size,
        }
    }

    fn allocate(&self) -> Option<Box<[u8; 256]>> {
        let current = self.allocated.load(Ordering::Relaxed);
        if current >= self.max_size {
            return None;
        }

        let mut pool = self.pool.lock();
        if let Some(buffer) = pool.pop() {
            Some(buffer)
        } else {
            self.allocated.fetch_add(256, Ordering::Relaxed);
            Some(Box::new([0u8; 256]))
        }
    }

    fn deallocate(&self, buffer: Box<[u8; 256]>) {
        self.pool.lock().push(buffer);
    }
}

/// String interning for common path components
pub struct StringInterner {
    interned: DashMap<CompactString, Arc<str>, ahash::RandomState>,
}

impl StringInterner {
    pub fn new() -> Self {
        let interner = Self {
            interned: DashMap::with_hasher(ahash::RandomState::new()),
        };

        // Pre-intern common components
        for component in &[
            "Windows", "System32", "Program Files", "Users", "Documents",
            "Downloads", "Desktop", "AppData", "Local", "Roaming", "Temp",
            "mnt", "c", "d", "cygdrive", "home", "usr", "bin", "lib",
        ] {
            interner.intern(component);
        }

        interner
    }

    pub fn intern(&self, s: &str) -> Arc<str> {
        let key = CompactString::from(s);
        if let Some(interned) = self.interned.get(&key) {
            return interned.clone();
        }

        let arc: Arc<str> = Arc::from(s);
        self.interned.insert(key, arc.clone());
        arc
    }

    pub fn get(&self, s: &str) -> Option<Arc<str>> {
        self.interned.get(&CompactString::from(s)).map(|v| v.clone())
    }
}

// Global string interner
lazy_static::lazy_static! {
    pub static ref STRING_INTERNER: StringInterner = StringInterner::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_level_cache() {
        let cache = PathCache::new(CacheConfig::default());

        // Test insertion and retrieval
        cache.insert("/mnt/c/test", "C:\\test".into(), 0, false);
        assert_eq!(cache.get("/mnt/c/test"), Some("C:\\test".into()));

        // Test L1 hit
        assert_eq!(cache.get("/mnt/c/test"), Some("C:\\test".into()));

        // Test stats
        assert!(cache.stats().l1_hits.load(Ordering::Relaxed) > 0);
    }

    #[test]
    fn test_bloom_filter() {
        let cache = PathCache::new(CacheConfig::default());

        // Add to negative cache
        cache.add_negative("/invalid/path");

        // Should not find in cache (bloom filter hit)
        assert_eq!(cache.get("/invalid/path"), None);

        if cache.config.enable_stats {
            assert!(cache.stats().bloom_hits.load(Ordering::Relaxed) > 0);
        }
    }

    #[test]
    fn test_string_interning() {
        let interner = StringInterner::new();

        let s1 = interner.intern("Windows");
        let s2 = interner.intern("Windows");

        // Should return the same Arc
        assert!(Arc::ptr_eq(&s1, &s2));
    }

    #[test]
    fn test_cache_eviction() {
        let mut config = CacheConfig::default();
        config.l2_size = 2;
        let cache = PathCache::new(config);

        cache.insert("/path1", "C:\\path1".into(), 0, false);
        cache.insert("/path2", "C:\\path2".into(), 0, false);
        cache.insert("/path3", "C:\\path3".into(), 0, false);

        // L2 cache should have evicted one entry
        assert!(cache.l2_cache.len() <= 2);
    }
}
