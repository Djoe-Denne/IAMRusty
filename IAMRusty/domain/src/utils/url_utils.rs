//! URL utility functions for building verification and other URLs

/// URL utility functions for simple URL operations
pub struct UrlUtils;

impl UrlUtils {
    /// URL encode a string value
    pub fn url_encode(value: &str) -> String {
        urlencoding::encode(value).to_string()
    }
    
    /// URL decode a string value  
    pub fn url_decode(value: &str) -> String {
        urlencoding::decode(value).unwrap_or_default().to_string()
    }

    /// Build a verification URL from email and token
    pub fn build_verification_url() -> String {
        format!("/api/auth/verify")
    }
} 