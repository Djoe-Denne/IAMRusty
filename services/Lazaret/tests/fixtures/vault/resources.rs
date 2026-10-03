//! Vault KV v2 wire types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Body for `GET /v1/{mount}/data/{path}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKvReadBody {
    pub data: VaultKvData,
}

/// Inner `data` envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultKvData {
    pub data: BTreeMap<String, String>,
}

impl VaultKvReadBody {
    /// One field payload.
    #[must_use]
    pub fn one_field(field: &str, value: &str) -> Self {
        let mut data = BTreeMap::new();
        data.insert(field.to_owned(), value.to_owned());
        Self {
            data: VaultKvData { data },
        }
    }
}
