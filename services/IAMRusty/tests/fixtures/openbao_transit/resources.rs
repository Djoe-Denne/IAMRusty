//! OpenBao Transit Sign request/response DTOs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitSignRequest {
    pub input: String,
    #[serde(default)]
    pub prehashed: bool,
    #[serde(default)]
    pub hash_algorithm: Option<String>,
    #[serde(default)]
    pub signature_algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitSignResponse {
    pub data: TransitSignData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitSignData {
    pub signature: String,
}
