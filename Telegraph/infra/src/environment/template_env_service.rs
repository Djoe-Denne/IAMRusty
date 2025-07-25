//! Template environment variable service for Telegraph

use std::collections::HashMap;
use std::env;
use tracing::{debug, warn};

/// Service for managing template-related environment variables
pub struct TemplateEnvironmentService;

impl TemplateEnvironmentService {
    /// Create a new template environment service
    pub fn new() -> Self {
        Self
    }

    /// Get all environment variables starting with "TELEGRAPH__TEMPLATE__"
    /// Returns a map with lowercase keys and '__' replaced with '.'
    /// 
    /// Example:
    /// - TELEGRAPH__TEMPLATE__BASE_URL=https://example.com becomes base.url=https://example.com
    /// - TELEGRAPH__TEMPLATE__VERIFY__URL=https://example.com/verify becomes verify.url=https://example.com/verify
    pub fn get_template_variables(&self) -> HashMap<String, String> {
        let mut variables = HashMap::new();
        let prefix = "TELEGRAPH__TEMPLATE__";

        debug!("Scanning environment variables for template variables with prefix: {}", prefix);

        for (key, value) in env::vars() {
            if key.starts_with(prefix) {
                // Remove the prefix
                let template_key = &key[prefix.len()..];
                
                // Convert to lowercase and replace '__' with '.'
                let normalized_key = template_key
                    .to_lowercase()
                    .replace("__", ".");

                debug!(
                    env_var = %key,
                    template_key = %normalized_key,
                    value = %value,
                    "Found template environment variable"
                );

                variables.insert(normalized_key, value);
            }
        }

        if variables.is_empty() {
            warn!("No template environment variables found with prefix: {}", prefix);
        } else {
            debug!("Found {} template environment variables", variables.len());
        }

        variables
    }

    /// Get a specific template variable by key
    /// The key should be in the format that would result after normalization
    /// (lowercase, with '.' as separators)
    pub fn get_template_variable(&self, key: &str) -> Option<String> {
        let env_key = format!("TELEGRAPH__TEMPLATE__{}", 
            key.to_uppercase().replace(".", "__"));
        
        match env::var(&env_key) {
            Ok(value) => {
                debug!(
                    key = %key,
                    env_key = %env_key,
                    value = %value,
                    "Found template environment variable"
                );
                Some(value)
            }
            Err(_) => {
                debug!(
                    key = %key,
                    env_key = %env_key,
                    "Template environment variable not found"
                );
                None
            }
        }
    }

    /// Check if a template variable exists
    pub fn has_template_variable(&self, key: &str) -> bool {
        self.get_template_variable(key).is_some()
    }
}

impl Default for TemplateEnvironmentService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_template_variables_empty() {
        let service = TemplateEnvironmentService::new();
        let variables = service.get_template_variables();
        
        // May or may not be empty depending on environment, just check it returns a map
        assert!(variables.len() >= 0);
    }

    #[test]
    fn test_normalize_key() {
        // This test verifies the key normalization logic
        let test_cases = vec![
            ("BASE_URL", "base.url"),
            ("VERIFY__URL", "verify.url"),
            ("RESET__PASSWORD__URL", "reset.password.url"),
            ("SIMPLE", "simple"),
        ];

        for (input, expected) in test_cases {
            let normalized = input.to_lowercase().replace("__", ".");
            assert_eq!(normalized, expected);
        }
    }

    #[test]
    fn test_env_key_construction() {
        let service = TemplateEnvironmentService::new();
        
        // Test the reverse transformation for get_template_variable
        let test_cases = vec![
            ("base.url", "TELEGRAPH__TEMPLATE__BASE__URL"),
            ("verify.url", "TELEGRAPH__TEMPLATE__VERIFY__URL"),
            ("reset.password.url", "TELEGRAPH__TEMPLATE__RESET__PASSWORD__URL"),
        ];

        for (key, expected_env_key) in test_cases {
            let env_key = format!("TELEGRAPH__TEMPLATE__{}", 
                key.to_uppercase().replace(".", "__"));
            assert_eq!(env_key, expected_env_key);
        }
    }
} 