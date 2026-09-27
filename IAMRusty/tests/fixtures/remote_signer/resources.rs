use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRequestBody {
    pub key_id: String,
    pub algorithm: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignResponseBody {
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKeyResponseBody {
    pub public_key: String,
}
