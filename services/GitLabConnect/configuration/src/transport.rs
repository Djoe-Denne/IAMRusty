//! Vendor transport exemptions must be explicit, never inferred from build mode.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VendorTransportSecurity {
    #[default]
    Verified,
    LocalInsecure,
    IsolatedTest,
}
