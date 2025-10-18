use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

pub struct BoxCache {
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl BoxCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
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
}

impl Default for BoxCache {
    fn default() -> Self {
        Self::new()
    }
}
