//! Envoy HTTP `ext_authz` Check: validate JWT, then emit `(iss, sub)` (ADR-0308).
//!
//! No `OpenFGA`. No events. Not a fifth hexagon.

#![allow(missing_docs)]

pub mod check;
pub mod config;
pub mod jwks_cache;

#[cfg(test)]
mod jwks_fixtures;

pub use check::{check_router, AuthzState};
pub use config::ExtAuthzConfig;
pub use jwks_cache::{JwksCache, KeyStatus};
