use lru::LruCache;
use once_cell::sync::Lazy;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

pub struct BoxCache {
    cache: Arc<RwLock<LruCache<String, String>>>,
    capacity: usize,
}

impl BoxCache {
    pub fn new() -> Self {
        let capacity = *DEFAULT_CACHE_CAPACITY;
        let lru = LruCache::new(NonZeroUsize::new(capacity).unwrap());
        Self {
            cache: Arc::new(RwLock::new(lru)),
            capacity,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(1);
        let lru = LruCache::new(NonZeroUsize::new(cap).unwrap());
        Self {
            cache: Arc::new(RwLock::new(lru)),
            capacity: cap,
        }
    }

    pub fn clear(&self) -> crate::mods::Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
        cache.clear();
        Ok(())
    }

    pub fn size(&self) -> crate::mods::Result<usize> {
        let cache = self
            .cache
            .read()
            .map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
        Ok(cache.len())
    }

    pub fn insert(&self, key: String, value: String) -> crate::mods::Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
        cache.put(key, value);
        Ok(())
    }

    pub fn get(&self, key: &str) -> crate::mods::Result<Option<String>> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
        Ok(cache.get(key).cloned())
    }
}

impl Default for BoxCache {
    fn default() -> Self {
        Self::new()
    }
}

// Default capacity can be overridden via environment in the future if needed
static DEFAULT_CACHE_CAPACITY: Lazy<usize> = Lazy::new(|| 256usize);
