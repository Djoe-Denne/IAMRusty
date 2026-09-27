//! WIF wiremock response DTOs (ADR-0307).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcpAccessTokenBody {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureAccessTokenBody {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
