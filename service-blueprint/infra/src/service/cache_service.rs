use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use {{SERVICE_NAME}}_domain::{CacheService, DomainError};

/// In-memory cache service for testing and development
pub struct InMemoryCacheService {
    storage: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryCacheService {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryCacheService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheService for InMemoryCacheService {
    async fn get(&self, key: &str) -> Result<Option<String>, DomainError> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.get(key).cloned())
    }

    async fn set(&self, key: &str, value: &str, _ttl_seconds: u32) -> Result<(), DomainError> {
        let mut storage = self.storage.lock().unwrap();
        storage.insert(key.to_string(), value.to_string());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), DomainError> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, DomainError> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.contains_key(key))
    }
}

/// Redis cache service implementation
#[cfg(feature = "redis")]
pub struct RedisCacheService {
    // Redis client would go here
}

#[cfg(feature = "redis")]
impl RedisCacheService {
    pub async fn new(_redis_url: &str) -> Result<Self, DomainError> {
        // Implementation would create Redis client
        Ok(Self {})
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl CacheService for RedisCacheService {
    async fn get(&self, _key: &str) -> Result<Option<String>, DomainError> {
        // Implementation would use Redis client
        todo!("Implement Redis get")
    }

    async fn set(&self, _key: &str, _value: &str, _ttl_seconds: u32) -> Result<(), DomainError> {
        // Implementation would use Redis client
        todo!("Implement Redis set with TTL")
    }

    async fn delete(&self, _key: &str) -> Result<(), DomainError> {
        // Implementation would use Redis client
        todo!("Implement Redis delete")
    }

    async fn exists(&self, _key: &str) -> Result<bool, DomainError> {
        // Implementation would use Redis client
        todo!("Implement Redis exists")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_cache_service() {
        let service = InMemoryCacheService::new();

        // Test set and get
        service.set("key1", "value1", 60).await.unwrap();
        let value = service.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Test exists
        let exists = service.exists("key1").await.unwrap();
        assert!(exists);

        let not_exists = service.exists("nonexistent").await.unwrap();
        assert!(!not_exists);

        // Test delete
        service.delete("key1").await.unwrap();
        let value_after_delete = service.get("key1").await.unwrap();
        assert_eq!(value_after_delete, None);

        let exists_after_delete = service.exists("key1").await.unwrap();
        assert!(!exists_after_delete);
    }

    #[tokio::test]
    async fn test_cache_service_multiple_keys() {
        let service = InMemoryCacheService::new();

        // Set multiple values
        service.set("key1", "value1", 60).await.unwrap();
        service.set("key2", "value2", 60).await.unwrap();
        service.set("key3", "value3", 60).await.unwrap();

        // Get all values
        assert_eq!(
            service.get("key1").await.unwrap(),
            Some("value1".to_string())
        );
        assert_eq!(
            service.get("key2").await.unwrap(),
            Some("value2".to_string())
        );
        assert_eq!(
            service.get("key3").await.unwrap(),
            Some("value3".to_string())
        );

        // Delete one and verify others remain
        service.delete("key2").await.unwrap();
        assert_eq!(
            service.get("key1").await.unwrap(),
            Some("value1".to_string())
        );
        assert_eq!(service.get("key2").await.unwrap(), None);
        assert_eq!(
            service.get("key3").await.unwrap(),
            Some("value3".to_string())
        );
    }
} 