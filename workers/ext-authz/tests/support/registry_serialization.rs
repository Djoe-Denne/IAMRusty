//! Real IAM publisher serialization, not manually invented trust metadata.
use iam_domain::entity::{signing_key::SigningKey, token::JwkSet};

pub fn serialize(keys: &[SigningKey]) -> String {
    serde_json::to_string(&JwkSet::from_registry_keys(keys)).expect("IAM JWKS serialization")
}
