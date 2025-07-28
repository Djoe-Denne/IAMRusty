use std::sync::Once;
use uuid::Uuid;

use {{SERVICE_NAME}}_configuration::{{SERVICE_NAME_PASCAL}}Config;
use {{SERVICE_NAME}}_domain::ExampleEntity;

static INIT: Once = Once::new();

/// Initialize test environment (logging, etc.)
pub fn init() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt::try_init();
    });
}

/// Create a test configuration
pub fn create_test_config() -> {{SERVICE_NAME_PASCAL}}Config {
    let mut config = {{SERVICE_NAME_PASCAL}}Config::default();
    
    // Use test database
    config.database.name = format!("{{SERVICE_NAME}}_test_{}", Uuid::new_v4());
    config.database.port = 0; // Random port for tests
    config.logging.level = "warn".to_string(); // Reduce test noise
    
    // Disable external services for tests
    config.queue.queue_type = "disabled".to_string();
    
    config
}

/// Create a test entity
pub fn create_test_entity(name: &str) -> ExampleEntity {
    ExampleEntity::new(
        name.to_string(),
        Some(format!("Test description for {}", name)),
    ).unwrap()
}

/// Create multiple test entities
pub fn create_test_entities(count: usize) -> Vec<ExampleEntity> {
    (0..count)
        .map(|i| create_test_entity(&format!("Test Entity {}", i + 1)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_config() {
        let config = create_test_config();
        assert!(config.database.name.contains("{{SERVICE_NAME}}_test_"));
        assert_eq!(config.database.port, 0);
        assert_eq!(config.logging.level, "warn");
    }

    #[test]
    fn test_create_test_entity() {
        let entity = create_test_entity("Test");
        assert_eq!(entity.name, "Test");
        assert_eq!(entity.description, Some("Test description for Test".to_string()));
    }

    #[test]
    fn test_create_test_entities() {
        let entities = create_test_entities(3);
        assert_eq!(entities.len(), 3);
        assert_eq!(entities[0].name, "Test Entity 1");
        assert_eq!(entities[1].name, "Test Entity 2");
        assert_eq!(entities[2].name, "Test Entity 3");
    }
} 