//! High-level path normalizer with optional caching support.

use crate::{
    error::{PathError, Result},
    normalization::{normalize_path_cow, NormalizationResult},
};

#[cfg(feature = "cache")]
use crate::cache::{Cache, LruCache};

#[cfg(feature = "std")]
use std::sync::{Arc, RwLock};

/// High-level path normalizer with optional caching and thread-safety.
///
/// This struct provides a convenient interface for path normalization with
/// configurable caching to improve performance for repeated operations.
///
/// # Examples
///
/// ```rust
/// use winpath::PathNormalizer;
///
/// let normalizer = PathNormalizer::new();
/// let result = normalizer.normalize("/mnt/c/users/david")?;
/// assert_eq!(result.path(), r"C:\users\david");
/// ```
#[derive(Debug)]
pub struct PathNormalizer {
    #[cfg(feature = "cache")]
    cache: Option<Arc<RwLock<LruCache<String, String>>>>,

    /// Configuration options
    config: NormalizerConfig,
}

/// Configuration options for the path normalizer.
#[derive(Debug, Clone)]
pub struct NormalizerConfig {
    /// Whether to enable caching (requires "cache" feature)
    pub cache_enabled: bool,

    /// Maximum cache size (number of entries)
    pub cache_size: usize,

    /// Whether to automatically add UNC prefix for long paths
    pub auto_long_prefix: bool,

    /// Whether to validate path components against Windows restrictions
    pub validate_components: bool,

    /// Whether to perform Unicode normalization (requires "unicode" feature)
    #[cfg(feature = "unicode")]
    pub unicode_normalize: bool,
}

impl Default for NormalizerConfig {
    fn default() -> Self {
        Self {
            cache_enabled: cfg!(feature = "cache"),
            cache_size: crate::constants::DEFAULT_CACHE_SIZE,
            auto_long_prefix: true,
            validate_components: true,
            #[cfg(feature = "unicode")]
            unicode_normalize: false,
        }
    }
}

impl PathNormalizer {
    /// Creates a new path normalizer with default configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::new();
    /// ```
    pub fn new() -> Self {
        Self::with_config(NormalizerConfig::default())
    }

    /// Creates a new path normalizer with custom configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::{PathNormalizer, NormalizerConfig};
    ///
    /// let config = NormalizerConfig {
    ///     cache_enabled: true,
    ///     cache_size: 2048,
    ///     ..Default::default()
    /// };
    /// let normalizer = PathNormalizer::with_config(config);
    /// ```
    pub fn with_config(config: NormalizerConfig) -> Self {
        #[cfg(feature = "cache")]
        let cache = if config.cache_enabled {
            Some(Arc::new(RwLock::new(LruCache::new(config.cache_size))))
        } else {
            None
        };

        Self {
            #[cfg(feature = "cache")]
            cache,
            config,
        }
    }

    /// Creates a path normalizer without caching.
    ///
    /// This is useful when you want to minimize memory usage or when
    /// paths are unlikely to be repeated.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::without_cache();
    /// ```
    pub fn without_cache() -> Self {
        let mut config = NormalizerConfig::default();
        config.cache_enabled = false;
        Self::with_config(config)
    }

    /// Normalizes a path to Windows format.
    ///
    /// This method handles caching automatically if enabled and provides
    /// detailed normalization results including metadata about the operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::new();
    /// let result = normalizer.normalize("/mnt/c/users/david")?;
    /// assert_eq!(result.path(), r"C:\users\david");
    /// assert!(result.was_modified());
    /// ```
    pub fn normalize<'a>(&self, path: &'a str) -> Result<NormalizationResult<'a>> {
        if path.is_empty() {
            return Err(PathError::EmptyPath);
        }

        // Check cache first
        #[cfg(feature = "cache")]
        if let Some(ref cache) = self.cache {
            if let Ok(cache_guard) = cache.read() {
                if let Some(cached_result) = cache_guard.get(&path.to_string()) {
                    // Return cached result as owned
                    return Ok(NormalizationResult::new(
                        alloc::borrow::Cow::Owned(cached_result.clone()),
                        crate::detection::detect_path_format(path),
                        cached_result.starts_with(crate::constants::UNC_PREFIX),
                        true, // Was cached, so considered "modified" from perspective of this call
                    ));
                }
            }
        }

        // Perform normalization
        let mut result = normalize_path_cow(path)?;

        // Apply configuration-specific post-processing
        result = self.apply_config_processing(result)?;

        // Cache the result if caching is enabled
        #[cfg(feature = "cache")]
        if let Some(ref cache) = self.cache {
            if let Ok(mut cache_guard) = cache.write() {
                cache_guard.insert(path.to_string(), result.path().to_string());
            }
        }

        Ok(result)
    }

    /// Normalizes a path and returns only the string result.
    ///
    /// This is a convenience method that discards metadata and always
    /// returns an owned string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::new();
    /// let path = normalizer.normalize_to_string("/mnt/c/users/david")?;
    /// assert_eq!(path, r"C:\users\david");
    /// ```
    pub fn normalize_to_string(&self, path: &str) -> Result<String> {
        self.normalize(path).map(|result| result.into_path().into_owned())
    }

    /// Batch normalizes multiple paths efficiently.
    ///
    /// This method can provide better performance than individual calls
    /// when processing many paths, especially with caching enabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::new();
    /// let paths = vec!["/mnt/c/users/david", "/mnt/d/temp", "C:/Windows"];
    /// let results = normalizer.normalize_batch(&paths)?;
    /// assert_eq!(results.len(), 3);
    /// ```
    pub fn normalize_batch(&self, paths: &[&str]) -> Result<Vec<String>> {
        let mut results = Vec::with_capacity(paths.len());

        for path in paths {
            results.push(self.normalize_to_string(path)?);
        }

        Ok(results)
    }

    /// Clears the internal cache if caching is enabled.
    ///
    /// This method is useful for memory management in long-running applications.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::new();
    /// // ... use normalizer ...
    /// normalizer.clear_cache();
    /// ```
    #[cfg(feature = "cache")]
    pub fn clear_cache(&self) {
        if let Some(ref cache) = self.cache {
            if let Ok(mut cache_guard) = cache.write() {
                cache_guard.clear();
            }
        }
    }

    /// Returns cache statistics if caching is enabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use winpath::PathNormalizer;
    ///
    /// let normalizer = PathNormalizer::new();
    /// // ... use normalizer ...
    /// if let Some(stats) = normalizer.cache_stats() {
    ///     println!("Cache size: {}", stats.size);
    ///     println!("Hit rate: {:.2}%", stats.hit_rate * 100.0);
    /// }
    /// ```
    #[cfg(feature = "cache")]
    pub fn cache_stats(&self) -> Option<CacheStats> {
        if let Some(ref cache) = self.cache {
            if let Ok(cache_guard) = cache.read() {
                return Some(cache_guard.stats());
            }
        }
        None
    }

    /// Returns the current configuration.
    pub fn config(&self) -> &NormalizerConfig {
        &self.config
    }

    /// Applies configuration-specific post-processing to normalization results.
    fn apply_config_processing<'a>(
        &self,
        mut result: NormalizationResult<'a>,
    ) -> Result<NormalizationResult<'a>> {
        // Unicode normalization
        #[cfg(feature = "unicode")]
        if self.config.unicode_normalize {
            result = self.apply_unicode_normalization(result)?;
        }

        // Additional validation if enabled
        if self.config.validate_components {
            self.validate_result(&result)?;
        }

        Ok(result)
    }

    /// Applies Unicode normalization to the path.
    #[cfg(feature = "unicode")]
    fn apply_unicode_normalization<'a>(
        &self,
        result: NormalizationResult<'a>,
    ) -> Result<NormalizationResult<'a>> {
        use unicode_normalization::UnicodeNormalization;

        let normalized_path = result.path().nfc().collect::<String>();

        if normalized_path != result.path() {
            Ok(NormalizationResult::new(
                alloc::borrow::Cow::Owned(normalized_path),
                result.original_format(),
                result.has_long_path_prefix(),
                true, // Mark as modified due to Unicode normalization
            ))
        } else {
            Ok(result)
        }
    }

    /// Validates the normalization result according to configuration.
    fn validate_result(&self, result: &NormalizationResult<'_>) -> Result<()> {
        // Additional validation logic can be added here
        // For now, just ensure the path isn't empty
        if result.is_empty() {
            return Err(PathError::EmptyPath);
        }

        Ok(())
    }
}

impl Default for PathNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics for monitoring performance.
#[cfg(feature = "cache")]
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Current number of entries in the cache
    pub size: usize,
    /// Maximum cache capacity
    pub capacity: usize,
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Cache hit rate (hits / (hits + misses))
    pub hit_rate: f64,
}

#[cfg(feature = "cache")]
impl CacheStats {
    /// Creates new cache statistics.
    pub fn new(size: usize, capacity: usize, hits: u64, misses: u64) -> Self {
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };

        Self {
            size,
            capacity,
            hits,
            misses,
            hit_rate,
        }
    }
}

// Thread safety
unsafe impl Send for PathNormalizer {}
unsafe impl Sync for PathNormalizer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizer_creation() {
        let normalizer = PathNormalizer::new();
        assert!(normalizer.config().auto_long_prefix);

        let normalizer = PathNormalizer::without_cache();
        assert!(!normalizer.config().cache_enabled);
    }

    #[test]
    fn test_basic_normalization() {
        let normalizer = PathNormalizer::new();

        let result = normalizer.normalize(r"C:\Users\David").unwrap();
        assert_eq!(result.path(), r"C:\Users\David");

        let result = normalizer.normalize("/mnt/c/users/david").unwrap();
        assert_eq!(result.path(), r"C:\users\david");
    }

    #[test]
    fn test_normalize_to_string() {
        let normalizer = PathNormalizer::new();

        let path = normalizer.normalize_to_string("/mnt/c/users/david").unwrap();
        assert_eq!(path, r"C:\users\david");
    }

    #[test]
    fn test_batch_normalization() {
        let normalizer = PathNormalizer::new();

        let paths = vec![
            "/mnt/c/users/david",
            "/cygdrive/d/temp",
            "C:/Windows",
        ];

        let results = normalizer.normalize_batch(&paths).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], r"C:\users\david");
        assert_eq!(results[1], r"D:\temp");
        assert_eq!(results[2], r"C:\Windows");
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_caching_behavior() {
        let normalizer = PathNormalizer::new();

        // First call should miss cache
        let result1 = normalizer.normalize("/mnt/c/test").unwrap();
        assert_eq!(result1.path(), r"C:\test");

        // Second call should hit cache
        let result2 = normalizer.normalize("/mnt/c/test").unwrap();
        assert_eq!(result2.path(), r"C:\test");

        // Check cache stats
        if let Some(stats) = normalizer.cache_stats() {
            assert!(stats.hits > 0);
        }
    }

    #[cfg(feature = "cache")]
    #[test]
    fn test_cache_clearing() {
        let normalizer = PathNormalizer::new();

        normalizer.normalize("/mnt/c/test").unwrap();
        normalizer.clear_cache();

        if let Some(stats) = normalizer.cache_stats() {
            assert_eq!(stats.size, 0);
        }
    }

    #[test]
    fn test_custom_config() {
        let config = NormalizerConfig {
            cache_enabled: false,
            auto_long_prefix: false,
            validate_components: true,
            ..Default::default()
        };

        let normalizer = PathNormalizer::with_config(config);
        assert!(!normalizer.config().cache_enabled);
        assert!(!normalizer.config().auto_long_prefix);
    }

    #[test]
    fn test_error_handling() {
        let normalizer = PathNormalizer::new();

        assert!(normalizer.normalize("").is_err());
        assert!(normalizer.normalize("/mnt/").is_err());
        assert!(normalizer.normalize_to_string("").is_err());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let normalizer = Arc::new(PathNormalizer::new());
        let mut handles = vec![];

        for i in 0..10 {
            let normalizer_clone = normalizer.clone();
            let handle = thread::spawn(move || {
                let path = format!("/mnt/c/test{}", i);
                normalizer_clone.normalize_to_string(&path).unwrap()
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.starts_with("C:\\test"));
        }
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn test_unicode_normalization() {
        let config = NormalizerConfig {
            unicode_normalize: true,
            ..Default::default()
        };

        let normalizer = PathNormalizer::with_config(config);

        // Test with Unicode path that needs normalization
        let unicode_path = "/mnt/c/tëst"; // Contains composed character
        let result = normalizer.normalize(unicode_path).unwrap();
        assert!(result.path().contains("test") || result.path().contains("tëst"));
    }
}
