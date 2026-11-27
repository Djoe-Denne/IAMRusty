//! Envoy HTTP `ext_authz` Check: validate JWT, then emit `(iss, sub)` (ADR-0308).
//!
//! No `OpenFGA`. No events. Not a fifth hexagon.

#![allow(missing_docs)]

mod authority;
pub mod check;
mod clock;
pub mod config;
mod iam_fallback;
pub mod jwks_cache;
mod postgres_replica;

#[cfg(test)]
mod jwks_fixtures;
mod replica_proof;

pub use check::{check_router, AuthzState};
pub use config::ExtAuthzConfig;
pub use jwks_cache::{JwksCache, KeyStatus};
