//! Response caching with TTL, key generation, and size-based eviction.
//!
//! Provides a cache for AI model responses keyed on prompt content and
//! model parameters, with configurable TTL and maximum size.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Configuration for the response cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled.
    pub enabled: bool,
    /// Maximum number of entries in the cache.
    pub max_entries: usize,
    /// Maximum total size in bytes for cached responses.
    pub max_size_bytes: usize,
    /// Time-to-live for cache entries.
    pub ttl: Duration,
    /// Whether to cache error responses.
    pub cache_errors: bool,
    /// Model IDs that should never be cached.
    pub excluded_models: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            max_size_bytes: 50 * 1024 * 1024, // 50 MB
            ttl: Duration::from_secs(3600),    // 1 hour
            cache_errors: false,
            excluded_models: Vec::new(),
        }
    }
}

/// A key for looking up cached responses.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Hash of the prompt/messages.
    pub prompt_hash: String,
    /// The model used.
    pub model: String,
    /// Temperature setting (discretized to avoid float comparison issues).
    pub temperature_millis: u32,
    /// Max tokens setting.
    pub max_tokens: Option<u32>,
}

impl CacheKey {
    /// Create a new cache key.
    pub fn new(prompt_hash: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            prompt_hash: prompt_hash.into(),
            model: model.into(),
            temperature_millis: 1000, // Default temperature 1.0
            max_tokens: None,
        }
    }

    /// Set the temperature (stored as millis to avoid float hashing).
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature_millis = (temp * 1000.0) as u32;
        self
    }

    /// Set the max tokens.
    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:t{}",
            &self.prompt_hash[..self.prompt_hash.len().min(8)],
            self.model,
            self.temperature_millis
        )
    }
}

/// Generate a simple hash string from prompt content.
pub fn hash_prompt(content: &str) -> String {
    // Simple FNV-1a-like hash for cache keying.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in content.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// A cached response entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The cache key.
    pub key: CacheKey,
    /// The cached response content.
    pub response: String,
    /// Size of the response in bytes.
    pub size_bytes: usize,
    /// When this entry was created.
    pub created_at: u64,
    /// When this entry was last accessed.
    pub last_accessed: u64,
    /// Number of times this entry has been accessed.
    pub access_count: u64,
    /// Token count of the cached response.
    pub response_tokens: Option<usize>,
    /// Whether this is a cached error.
    pub is_error: bool,
}

impl CacheEntry {
    /// Create a new cache entry.
    pub fn new(key: CacheKey, response: impl Into<String>) -> Self {
        let response = response.into();
        let size = response.len();
        let now = current_epoch_secs();
        Self {
            key,
            response,
            size_bytes: size,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            response_tokens: None,
            is_error: false,
        }
    }

    /// Check if this entry has expired given a TTL.
    pub fn is_expired(&self, ttl: Duration) -> bool {
        let now = current_epoch_secs();
        now - self.created_at > ttl.as_secs()
    }

    /// Mark this entry as accessed.
    pub fn touch(&mut self) {
        self.last_accessed = current_epoch_secs();
        self.access_count += 1;
    }

    /// Age of this entry in seconds.
    pub fn age_secs(&self) -> u64 {
        current_epoch_secs() - self.created_at
    }
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// In-memory response cache with TTL and size-based eviction.
#[derive(Debug, Clone)]
pub struct ResponseCache {
    /// Cache configuration.
    config: CacheConfig,
    /// Cached entries indexed by key.
    entries: HashMap<CacheKey, CacheEntry>,
    /// Cache statistics.
    stats: CacheStats,
}

/// Cache hit/miss statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cache lookups.
    pub lookups: u64,
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Entries evicted due to TTL.
    pub ttl_evictions: u64,
    /// Entries evicted due to size limits.
    pub size_evictions: u64,
    /// Total bytes currently cached.
    pub current_bytes: usize,
    /// Total entries currently cached.
    pub current_entries: usize,
}

impl CacheStats {
    /// Return the hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        if self.lookups == 0 {
            return 0.0;
        }
        (self.hits as f64 / self.lookups as f64) * 100.0
    }

    /// Return the miss rate as a percentage.
    pub fn miss_rate(&self) -> f64 {
        100.0 - self.hit_rate()
    }
}

impl fmt::Display for CacheStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} entries ({} bytes), {}/{} hits ({:.1}%)",
            self.current_entries,
            self.current_bytes,
            self.hits,
            self.lookups,
            self.hit_rate()
        )
    }
}

impl ResponseCache {
    /// Create a new cache with the given configuration.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: HashMap::new(),
            stats: CacheStats::default(),
        }
    }

    /// Create a cache with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Look up a cached response.
    pub fn get(&mut self, key: &CacheKey) -> Option<&str> {
        self.stats.lookups += 1;

        // Check if model is excluded.
        if self.config.excluded_models.contains(&key.model) {
            self.stats.misses += 1;
            return None;
        }

        if let Some(entry) = self.entries.get_mut(key) {
            if entry.is_expired(self.config.ttl) {
                self.stats.ttl_evictions += 1;
                self.stats.misses += 1;
                // Can't remove during immutable borrow, mark for later cleanup.
                return None;
            }
            entry.touch();
            self.stats.hits += 1;
            Some(&entry.response)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Insert a response into the cache.
    pub fn put(&mut self, key: CacheKey, response: impl Into<String>) {
        if !self.config.enabled {
            return;
        }
        if self.config.excluded_models.contains(&key.model) {
            return;
        }

        let entry = CacheEntry::new(key.clone(), response);
        let entry_size = entry.size_bytes;

        // Evict if at capacity.
        while self.entries.len() >= self.config.max_entries {
            self.evict_oldest();
        }
        while self.stats.current_bytes + entry_size > self.config.max_size_bytes {
            if !self.evict_oldest() {
                break;
            }
        }

        self.stats.current_bytes += entry_size;
        self.stats.current_entries = self.entries.len() + 1;
        self.entries.insert(key, entry);
    }

    /// Remove a specific entry.
    pub fn remove(&mut self, key: &CacheKey) -> bool {
        if let Some(entry) = self.entries.remove(key) {
            self.stats.current_bytes -= entry.size_bytes;
            self.stats.current_entries = self.entries.len();
            true
        } else {
            false
        }
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats.current_bytes = 0;
        self.stats.current_entries = 0;
    }

    /// Remove all expired entries.
    pub fn cleanup(&mut self) {
        let ttl = self.config.ttl;
        let before = self.entries.len();
        self.entries.retain(|_, entry| {
            if entry.is_expired(ttl) {
                self.stats.current_bytes -= entry.size_bytes;
                self.stats.ttl_evictions += 1;
                false
            } else {
                true
            }
        });
        self.stats.current_entries = self.entries.len();
        let _ = before; // suppress unused warning
    }

    /// Return cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Return the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Evict the oldest (least recently accessed) entry. Returns false if empty.
    fn evict_oldest(&mut self) -> bool {
        let oldest_key = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            if let Some(entry) = self.entries.remove(&key) {
                self.stats.current_bytes -= entry.size_bytes;
                self.stats.size_evictions += 1;
                self.stats.current_entries = self.entries.len();
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_prompt_deterministic() {
        let h1 = hash_prompt("hello world");
        let h2 = hash_prompt("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_prompt_different() {
        let h1 = hash_prompt("hello");
        let h2 = hash_prompt("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn cache_put_get() {
        let mut cache = ResponseCache::with_defaults();
        let key = CacheKey::new(hash_prompt("test"), "claude-sonnet-4");
        cache.put(key.clone(), "response text");
        let result = cache.get(&key);
        assert_eq!(result, Some("response text"));
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn cache_miss() {
        let mut cache = ResponseCache::with_defaults();
        let key = CacheKey::new("nonexistent", "model");
        let result = cache.get(&key);
        assert!(result.is_none());
        assert_eq!(cache.stats().misses, 1);
    }

    #[test]
    fn cache_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            ..Default::default()
        };
        let mut cache = ResponseCache::new(config);
        cache.put(CacheKey::new("a", "m"), "resp1");
        cache.put(CacheKey::new("b", "m"), "resp2");
        cache.put(CacheKey::new("c", "m"), "resp3");
        assert_eq!(cache.len(), 2);
    }
}
