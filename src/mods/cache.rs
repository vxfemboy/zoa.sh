use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

pub struct BoxCache {
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl BoxCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_or_compute<F>(&self, key: &str, compute: F) -> crate::mods::Result<String>
    where
        F: FnOnce() -> crate::mods::Result<String>,
    {
        // Try to read from cache first
        {
            let cache = self.cache.read().map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
            if let Some(cached) = cache.get(key) {
                return Ok(cached.clone());
            }
        }

        // Compute the value
        let value = compute()?;

        // Store in cache
        {
            let mut cache = self.cache.write().map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
            cache.insert(key.to_string(), value.clone());
        }

        Ok(value)
    }

    pub fn clear(&self) -> crate::mods::Result<()> {
        let mut cache = self.cache.write().map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
        cache.clear();
        Ok(())
    }

    pub fn size(&self) -> crate::mods::Result<usize> {
        let cache = self.cache.read().map_err(|_| crate::mods::AppError::Server("Cache lock poisoned".to_string()))?;
        Ok(cache.len())
    }
}

impl Default for BoxCache {
    fn default() -> Self {
        Self::new()
    }
}
